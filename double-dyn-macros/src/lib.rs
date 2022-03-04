
use std::collections::HashMap;

use proc_macro2::token_stream::IntoIter as TokenIter;
use proc_macro2::{*};
use quote::{quote};
use heck::AsSnakeCase;

mod parse;
use crate::parse::*;

#[proc_macro]
pub fn double_dyn_fn(input: proc_macro::TokenStream) -> proc_macro::TokenStream {

    let output = match double_dyn_fn_internal(input.into()) {
        Ok(expanded) => expanded,
        Err(error) => error.into_compile_error(),
    };

    output.into()
}

fn double_dyn_fn_internal(input: TokenStream) -> Result<TokenStream, SyntaxError> {

    //==================================================================================================================
    // PHASE 1: Parse the Macro Invocation
    //==================================================================================================================

    //Parse the preamble of the invocation to get the trait names and any trait bounds
    let mut iter = input.into_iter();
    require_keyword(&mut iter, "type", Span::call_site())?;
    require_keyword(&mut iter, "A", Span::call_site())?;
    require_punct(&mut iter, ':', Span::call_site())?;
    let trait_a_name = require_ident(&mut iter, Span::call_site())?;
    let mut trait_a_bounds = TokenStream::new();
    while !if_punct(&iter, ';')? {
        let token = next_token(&mut iter, Span::call_site())?;
        trait_a_bounds.extend([token]);
    }
    require_punct(&mut iter, ';', Span::call_site())?;
    require_keyword(&mut iter, "type", Span::call_site())?;
    require_keyword(&mut iter, "B", Span::call_site())?;
    require_punct(&mut iter, ':', Span::call_site())?;
    let trait_b_name = require_ident(&mut iter, Span::call_site())?;
    let mut trait_b_bounds = TokenStream::new();
    while !if_punct(&iter, ';')? {
        let token = next_token(&mut iter, Span::call_site())?;
        trait_b_bounds.extend([token]);
    }
    require_punct(&mut iter, ';', Span::call_site())?;

    //See if both the A and B traits are the same, because that affects several behaviors later on
    let single_trait = trait_a_name.to_string() == trait_b_name.to_string();

    //The pub qualifiers must match across every function signature
    let mut pub_qualifiers = TokenStream::new();

    //Parse each function signature
    let mut first_sig = true;
    let mut fn_sigs = HashMap::new();
    loop {
        let mut temp_iter = iter.clone();
        match require_fn_signature(&mut temp_iter, true, Span::call_site()) {
            Ok(sig) => {

                //Check that every arg has an arg name
                for arg in sig.args.iter() {
                    if arg.arg_name.is_none() {
                        return Err(SyntaxError {
                            message: format!("missing arg name.  anonymous args are not allowed"),
                            span: arg.arg_type.clone().into_iter().next().unwrap().span(),
                        });
                    }
                }

                //Check for duplicate function signature names
                if fn_sigs.get(&sig.fn_name.to_string()).is_some() {
                    return Err(SyntaxError {
                        message: format!("duplicate functions not allowed"),
                        span: sig.fn_name.span(),
                    });    
                }

                //Check that the pub qualifiers match across every function signature
                if first_sig {
                    pub_qualifiers = sig.pub_qualifiers.clone();
                    first_sig = false;
                } else {
                    if tokens_to_string(pub_qualifiers.clone()) != tokens_to_string(sig.pub_qualifiers.clone()) {
                        return Err(SyntaxError {
                            message: format!("All functions must have the same visibility (e.g. 'pub')"),
                            span: sig.fn_name.span(),
                        });
                    }
                }

                //Identify the arg indices that might be A or B
                let mut possible_a_args = vec![];
                let mut possible_b_args = vec![];
                for (i, arg) in sig.args.iter().enumerate() {
                    let arg_token_iter = arg.arg_type.clone().into_iter();
                    if if_contains_sequence(&arg_token_iter, &["dyn", &trait_a_name.to_string()])? {
                        possible_a_args.push(i);
                    }
                    if if_contains_sequence(&arg_token_iter, &["dyn", &trait_b_name.to_string()])? {
                        possible_b_args.push(i);
                    }
                }

                //If we didn't identify at least one potential A arg index and one potential B then it's an error
                if possible_a_args.len() < 1 || possible_b_args.len() < 1 {
                    return Err(SyntaxError {
                        message: format!("function must have at least one dyn A and one dyn B argument"),
                        span: sig.fn_name.span(),
                    });    
                }

                //Add our valid sig to the map, and move on
                fn_sigs.insert(sig.fn_name.to_string(), (sig, possible_a_args, possible_b_args));
                iter = temp_iter;
            },
            Err(err) => {
                if fn_sigs.len() > 0 {
                    //See if we're ready to move on to implementations
                    if if_keyword(&mut iter, "impl")? || if_punct(&mut iter, '#')? {
                        //NOTE: currently we only have #attributes for impls.  This logic will need to change
                        // if we end up needing to support attributes for functions
                        break;
                    } else {
                        return Err(err); //We found some other error in the function signature
                    }
                } else {
                    return Err(err); //We need at least one function signature
                }
            }
        }
    }

    //Parse each type pair impl block
    let mut pairs_map = HashMap::new();
    let mut type_a_map = HashMap::new();
    let mut type_b_map = HashMap::new();
    loop {
        let mut impl_fns = HashMap::new();

        // Check for any attributes (specifically #[commutative])
        let is_commutative = if if_punct(&mut iter, '#')? {
            require_punct(&mut iter, '#', Span::call_site())?;
            let attrib_group = require_group(&mut iter, Delimiter::Bracket, Span::call_site(), "expected square brackets")?;
            let mut attrib_token_iter = attrib_group.stream().into_iter();

            //Only the "commutative" attribute is supported
            require_keyword(&mut attrib_token_iter, "commutative", attrib_group.span())?;

            //"commutative" is only compative if the A and B traits are the same trait
            if !single_trait {
                return Err(SyntaxError {
                    message: format!("commutative attribute requires matching A and B traits"),
                    span: attrib_group.span(),
                });
            }

            true
        } else {
            false
        };

        // The preamble, e.g. "impl for <TypeA, TypeB>"
        require_keyword(&mut iter, "impl", Span::call_site())?;
        require_keyword(&mut iter, "for", Span::call_site())?;
        let type_pair_group = require_angle_group(&mut iter, Span::call_site(), "expected type pair in angle brackets")?;
        let mut pair_token_iter = type_pair_group.interior_tokens.into_iter();

        //We support either a type by itself or a list of types in square brackets
        let type_a_list = require_type_or_type_list(&mut pair_token_iter, type_pair_group.close_bracket.span())?;
        if !if_punct(&mut pair_token_iter, ',')? { //So the error message is a little better
            return Err(SyntaxError {
                message: format!("expected type or type list for 'B'"),
                span: type_pair_group.close_bracket.span(),
            });
        }
        require_punct(&mut pair_token_iter, ',', type_pair_group.close_bracket.span())?;
        let type_b_list = require_type_or_type_list(&mut pair_token_iter, type_pair_group.close_bracket.span())?;

        // The block containing the functions
        let fn_group = require_group(&mut iter, Delimiter::Brace, Span::call_site(), "expected curly braces for fn impls")?;
        let mut block_token_iter = fn_group.stream().into_iter();
        while !if_end(&mut block_token_iter)? {
            let sig = require_fn_signature(&mut block_token_iter, false, fn_group.span())?;
            let fn_body = require_group(&mut block_token_iter, Delimiter::Brace, fn_group.span(), "expected fn body")?;
            
            //Check for duplicate function names
            if impl_fns.get(&sig.fn_name.to_string()).is_some() {
                return Err(SyntaxError {
                    message: format!("duplicate functions not allowed"),
                    span: sig.fn_name.span(),
                });
            }

            //Check that this implementation name matches one of the signatures defined above
            if let Some((_template_sig, possible_a_args, possible_b_args)) = fn_sigs.get_mut(&sig.fn_name.to_string()) {

                //Make sure we can correlate the arg positions for the A and B types
                for (i, arg) in sig.args.iter().enumerate() {
                    let arg_token_iter = arg.arg_type.clone().into_iter();

                    //We're looking for either an "#A" or the concrete A type itself in the case that we only have one possible A type
                    if !if_contains_sequence(&arg_token_iter, &["#", "A"])? 
                    && !if_contains_tokens(&arg_token_iter, type_a_list[0].clone().into_iter())? {
                        //If this arg isn't a candidate for a type_a, make sure it's not in the possible_a_args list
                        if let Some(idx) = possible_a_args.iter().position(|&el| el == i) {
                            possible_a_args.remove(idx);
                        }
                    }
                    //Do the same for B args
                    if !if_contains_sequence(&arg_token_iter, &["#", "B"])? 
                    && !if_contains_tokens(&arg_token_iter, type_b_list[0].clone().into_iter())? {
                        //If this arg isn't a candidate for a type_a, make sure it's not in the possible_a_args list
                        if let Some(idx) = possible_b_args.iter().position(|&el| el == i) {
                            possible_b_args.remove(idx);
                        }
                    }
                }

                //If we ended up disqualifying every arg then that's a problem
                if possible_a_args.len() < 1 {
                    return Err(SyntaxError {
                        message: format!("can't infer position of A arg when reconciled with fn signature"),
                        span: sig.fn_name.span(),
                    });
                }
                if possible_b_args.len() < 1 {
                    return Err(SyntaxError {
                        message: format!("can't infer position of B arg when reconciled with fn signature"),
                        span: sig.fn_name.span(),
                    });
                }

                impl_fns.insert(sig.fn_name.to_string(), (sig, fn_body));
            } else {
                return Err(SyntaxError {
                    message: format!("matching fn signature not found"),
                    span: sig.fn_name.span(),
                });
            }
        }

        //Check that every function has been implemented
        if impl_fns.len() != fn_sigs.len() {
            return Err(SyntaxError {
                message: format!("incomplete implementation of declared functions"),
                span: fn_group.span(),
            });
        }

        //Put a pair record in the HashMap for each type_a-type_b pair
        for type_a in type_a_list.iter() {
            let type_a_string = format!("{}", AsSnakeCase(tokens_to_string(type_a.clone())));

            for type_b in type_b_list.iter() {
                let type_b_string = format!("{}", AsSnakeCase(tokens_to_string(type_b.clone())));

                //Go over each fn implementation, and replace the placeholders with the concrete types
                let mut updated_fns = HashMap::new();
                for (fn_name, (sig, fn_body)) in impl_fns.iter() {

                    //Go through the args in the function signature and swap out the #A and #B types
                    let mut new_sig = sig.clone();
                    for arg in new_sig.args.iter_mut() {
                        let new_arg_type = replace_type_placeholders(arg.arg_type.clone(), type_a, type_b)?;
                        arg.arg_type = new_arg_type;
                    }

                    //Now do the same thing for the function body
                    let new_fn_body = replace_type_placeholders(fn_body.stream(), type_a, type_b)?;

                    updated_fns.insert(fn_name.clone(), (new_sig, new_fn_body));
                }

                //Put the pair in the pairs_map
                pairs_map
                    .entry(type_a_string.clone())
                    .and_modify(|type_b_map : &mut HashMap<String, HashMap<String, (FnSignature, TokenStream)>>| {
                        type_b_map.insert(type_b_string.clone(), updated_fns.clone()); //NOTE: these clones bug me but the compiler probably takes care of them
                    })
                    .or_insert({
                        let mut new_map = HashMap::with_capacity(1);
                        new_map.insert(type_b_string.clone(), updated_fns);
                        new_map
                    });

                //If the pair is_commutative, then put the inverse in the pairs_map as well
                if is_commutative {

                    //We need to do the #A and #B swap in reverse
                    let mut updated_fns = HashMap::new();
                    for (fn_name, (sig, fn_body)) in impl_fns.iter() {
    
                        //Go through the args in the function signature and swap out the #A and #B types
                        let mut new_sig = sig.clone();
                        for arg in new_sig.args.iter_mut() {
                            let new_arg_type = replace_type_placeholders(arg.arg_type.clone(), type_b, type_a)?;
                            arg.arg_type = new_arg_type;
                        }
    
                        //Now do the same thing for the function body
                        let new_fn_body = replace_type_placeholders(fn_body.stream(), type_b, type_a)?;
    
                        updated_fns.insert(fn_name.clone(), (new_sig, new_fn_body));
                    }
    
                    //Put the inverse pair in the pairs_map
                    pairs_map
                        .entry(type_b_string.clone())
                        .and_modify(|type_a_map : &mut HashMap<String, HashMap<String, (FnSignature, TokenStream)>>| {
                            type_a_map.insert(type_a_string.clone(), updated_fns.clone());
                        })
                        .or_insert({
                            let mut new_map = HashMap::with_capacity(1);
                            new_map.insert(type_a_string.clone(), updated_fns);
                            new_map
                        });
                }

                //Update the map of all b_types
                type_b_map.insert(type_b_string, type_b.clone());
            }

            //Update the map of all a_types
            type_a_map.insert(type_a_string, type_a.clone());
        }

        //Any more tokens must be additional impl blocks
        if if_end(&mut iter)? {
            break;
        }
    }

    //For each function, collapse the possible arg positions (possible_a_args & possible_b_args) into a single arg index
    for (sig, possible_a_args, possible_b_args) in fn_sigs.values_mut() {

        //If we have the same trait for A and B, then just pick one index for A and the other one for B
        if single_trait {
            if let Some(idx) = possible_b_args.iter().position(|&el| el == possible_a_args[0]) {
                possible_b_args.remove(idx);
            }
            if let Some(idx) = possible_a_args.iter().position(|&el| el == possible_b_args[0]) {
                possible_a_args.remove(idx);
            }
        }

        //And if we ended up disqualifying all possible args, that's an error
        if possible_a_args.len() < 1 || possible_b_args.len() < 1 {
            return Err(SyntaxError {
                message: format!("can't infer position of both A and B args"),
                span: sig.fn_name.span(),
            });
        }

        //Now if we have more than one index for for either A or B then the signature is ambiguous so that's an error
        if possible_a_args.len() > 1 {
            return Err(SyntaxError {
                message: format!("ambiguous signature; can't infer position of A arg"),
                span: sig.fn_name.span(),
            });
        }
        if possible_b_args.len() > 1 {
            return Err(SyntaxError {
                message: format!("ambiguous signature; can't infer position of B arg"),
                span: sig.fn_name.span(),
            });
        }
    }
    
    //==================================================================================================================
    // PHASE 2: Build the Output Tokens
    //==================================================================================================================

    //If we're only dealing with one trait, then we only have one list of types
    if single_trait {
        type_a_map.extend(type_b_map.iter().map(|pair| (pair.0.clone(), pair.1.clone())));
        type_b_map = type_a_map.clone();
    }
    
    //Transmute all of the function prototypes into methods for the ATrait
    let mut l1_sig_tokens = TokenStream::new();
    let mut l1_sigs = HashMap::new();
    for (fn_name, (sig, possible_a_args, _possible_b_args)) in fn_sigs.iter() {

        //Turns "fn min_max(val: i32, min: &dyn MyTraitA, max: &dyn MyTraitB) -> Result<i32, String>;" into
        // "fn l1_min_max(&self, val: i32, max: &dyn MyTraitB) -> Result<i32, String>;"
        let mut new_sig = sig.clone();
        new_sig.pub_qualifiers = TokenStream::new(); //no visibility qualifiers on trait methods
        new_sig.fn_name = Ident::new(&format!("l1_{}", fn_name), sig.fn_name.span());
        new_sig.args.remove(possible_a_args[0]); //Get rid of the arg that'll be replaced by self
        new_sig.args.insert(0, FnArg{
            arg_name: None,
            arg_type: quote! { &self }
        });

        let sig_tokens = render_fn_signature(new_sig.clone())?;
        l1_sigs.insert(fn_name.clone(), (new_sig, sig_tokens.clone()));
        l1_sig_tokens.extend(sig_tokens);
        l1_sig_tokens.extend(quote! { ; });    
    }

    //Transmute all of the function prototypes into methods for the BTrait
    let mut l2_sig_tokens = TokenStream::new();
    let mut l2_sigs = HashMap::new();
    for (fn_name, (sig, possible_a_args, possible_b_args)) in fn_sigs.iter() {

        //Create a separate variant of each function for each of the A types
        for a_type_string in type_a_map.keys() {
            
            //Turns "fn min_max(val: i32, min: &dyn MyTraitA, max: &dyn MyTraitB) -> Result<i32, String>;" into
            // "fn l2_min_max_i32(&self, val: i32, min: &i32) -> Result<i32, String>;"
            let mut new_sig = sig.clone();
            new_sig.pub_qualifiers = TokenStream::new(); //no visibility qualifiers on trait methods
            new_sig.fn_name = Ident::new(&format!("l2_{}_{}", fn_name, a_type_string), sig.fn_name.span());
            //Remove the A and B args because we'll replace them.  But we need to remove them in the right order
            // because we don't want to screw up the indices
            let old_a_arg = if possible_a_args[0] < possible_b_args[0] {
                new_sig.args.remove(possible_b_args[0]);
                new_sig.args.remove(possible_a_args[0])
            } else {
                let old_a_arg = new_sig.args.remove(possible_a_args[0]);
                new_sig.args.remove(possible_b_args[0]);
                old_a_arg
            };
            new_sig.args.insert(0, FnArg{
                arg_name: None,
                arg_type: quote! { &self }
            });
            let type_a_tokens = type_a_map.get(a_type_string).unwrap().clone();
            new_sig.args.push(FnArg{
                arg_name: old_a_arg.arg_name,
                arg_type: quote! { &#type_a_tokens }
            });

            let sig_tokens = render_fn_signature(new_sig.clone())?;
            l2_sigs.insert((fn_name, a_type_string), (new_sig, sig_tokens.clone()));
            l2_sig_tokens.extend(sig_tokens);
            l2_sig_tokens.extend(quote! { ; });    
        }
    }

    // --1-- Create the definition of the traits
    let mut result_tokens = if single_trait {
        quote! {
            #pub_qualifiers trait #trait_a_name #trait_a_bounds {
                #l1_sig_tokens

                #l2_sig_tokens
            }
        }
    } else {
        quote! {
            #pub_qualifiers trait #trait_a_name #trait_a_bounds {
                #l1_sig_tokens
            }

            #pub_qualifiers trait #trait_b_name #trait_b_bounds {
                #l2_sig_tokens
            }
        }
    };

    // --2-- Emit the L1 trait impls
    for (a_type_name, a_type) in type_a_map.iter() {

        //If we only have one trait, build up the l2 fn impls, to be included alongside the l1 fn impls
        // Since the a-types and b-types are the same set of types, we need to provide impls for the whole set so
        // we're passing the b_type for A and the a_type for B.
        let l2_impls_single_trait = if single_trait {
            let mut l2_impls = TokenStream::new();
            for b_type_name in type_b_map.keys() {
                let impl_tokens = render_l2_fns_for_pair(b_type_name, a_type_name, &pairs_map, &fn_sigs, &l2_sigs)?;
                l2_impls.extend(impl_tokens);
            }
            l2_impls
        } else {
            TokenStream::new()
        };
    
        //Build up the tokens for the l1 methods, for the "impl TraitA for TypeA"
        let mut l1_impls = TokenStream::new();
        for (orig_fn_name, (_l1_sig, l1_sig_tokens)) in l1_sigs.iter() {
            let (prototype_sig, possible_a_args, possible_b_args) = fn_sigs.get(orig_fn_name).unwrap();

            //Get the name of the B arg, so we can use it to call the l2 function
            let b_arg_name = prototype_sig.args[possible_b_args[0]].arg_name.clone().unwrap();

            //We'll pass all of the other args to the l2 function
            let mut other_arg_name_tokens = TokenStream::new();
            for (i, arg) in prototype_sig.args.iter().enumerate() {
                if i != possible_a_args[0] && i != possible_b_args[0] {
                    let arg_name = arg.arg_name.clone().unwrap();
                    other_arg_name_tokens.extend(quote! {
                        #arg_name,
                    });
                }
            }

            //Figure out the l2 function name
            let (l2_sig, _l2_sig_tokens) = l2_sigs.get(&(orig_fn_name, &a_type_name)).unwrap();
            let l2_fn_name = &l2_sig.fn_name;

            //Compose an l1 function that calls the appropriate l2 function with the right args
            let l1_impl = quote! {
                #l1_sig_tokens {
                    #b_arg_name.#l2_fn_name(#other_arg_name_tokens &self)
                }
            };

            l1_impls.extend(l1_impl);
        }

        let a_trait_impl = quote! {
            impl #trait_a_name for #a_type {
                #l1_impls
                #l2_impls_single_trait
            }
        };

        result_tokens.extend(a_trait_impl);
    }

    // --3-- Emit the L2 trait impls
    if !single_trait {
        for (b_type_name, b_type) in type_b_map.iter() {

            let mut l2_impls = TokenStream::new();
            for a_type_name in type_a_map.keys() {
                let impl_tokens = render_l2_fns_for_pair(a_type_name, b_type_name, &pairs_map, &fn_sigs, &l2_sigs)?;
                l2_impls.extend(impl_tokens);
            }

            let b_trait_impl = quote! {
                impl #trait_b_name for #b_type {
                    #l2_impls
                }
            };
    
            result_tokens.extend(b_trait_impl);
        }
    }

    // --4-- Emit the top-level function(s)
    for (orig_fn_name, (sig, possible_a_args, _possible_b_args)) in fn_sigs.iter() {

        let sig_tokens = render_fn_signature(sig.clone())?;
        let (l1_sig, _l1_sig_tokens) = l1_sigs.get(orig_fn_name).unwrap();
        let l1_fn_name = l1_sig.fn_name.clone();

        //Get the name of the A arg, so we can use it to call the l1 trait method
        let a_arg_name = sig.args[possible_a_args[0]].arg_name.clone().unwrap();

        //We'll pass all of the other args to the l1 method
        let mut other_arg_name_tokens = TokenStream::new();
        for (i, arg) in sig.args.iter().enumerate() {
            if i != possible_a_args[0] {
                let arg_name = arg.arg_name.clone().unwrap();
                other_arg_name_tokens.extend(quote! {
                    #arg_name,
                });
            }
        }

        let fn_tokens = quote! {
            #sig_tokens {
                #a_arg_name.#l1_fn_name(#other_arg_name_tokens)
            }
        };

        result_tokens.extend(fn_tokens);
    }

    Ok(result_tokens.into())
}

//Parse a type by itself or a list of types in square brackets
fn require_type_or_type_list(iter: &mut TokenIter, err_span: Span) -> Result<Vec<TokenStream>, SyntaxError> {
    
    let mut type_list = vec![];
    if if_group(iter, Delimiter::Bracket)? {
        let type_list_group = require_group(iter, Delimiter::Bracket, err_span.clone(), "expected square braces for type array")?;
        let mut type_tokens_iter = type_list_group.stream().into_iter();
        loop {
            type_list.push(require_type(&mut type_tokens_iter, type_list_group.span())?);
            if if_end(&type_tokens_iter)? {
                break;
            } else {
                require_punct(&mut type_tokens_iter, ',', type_list_group.span())?;
            }
        }
        if type_list.len() < 1 {
            //return err if we didn't push anything to the array
            return Err(syntax(TokenTree::Group(type_list_group), "expected at least one type"));
        }
    } else {
        let type_group = require_type(iter, err_span.clone())?;
        type_list.push(type_group);
    }

    Ok(type_list)
}

fn render_l2_fns_for_pair(
    a_type_name: &String,
    b_type_name: &String,
    pairs_map: &HashMap<String, HashMap<String, HashMap<String, (FnSignature, TokenStream)>>>,
    fn_sigs: &HashMap<String, (FnSignature, Vec<usize>, Vec<usize>)>,
    l2_sigs: &HashMap<(&String, &String), (FnSignature, TokenStream)>) -> Result<TokenStream, SyntaxError> {

    let mut l2_impls = TokenStream::new();

    let found_pair = if let Some(a_pair_map) = pairs_map.get(a_type_name) {
        if let Some(pair_fn_map) = a_pair_map.get(b_type_name) {

            //Emit methods with the body from the macro invocation 
            for (orig_fn_name, (_sig, _possible_a_args, possible_b_args)) in fn_sigs.iter() {

                let (pair_fn_sig, pair_fn_body) = pair_fn_map.get(orig_fn_name).unwrap();

                //Turns "fn min_max(val: i32, min: &i32) -> Result<i32, String>;" into
                // "fn l2_min_max_i32(&self, val: i32, min: &i32) -> Result<i32, String>;"
                let mut new_sig = pair_fn_sig.clone();
                new_sig.fn_name = Ident::new(&format!("l2_{}_{}", orig_fn_name, a_type_name), pair_fn_sig.fn_name.span());
                //Remove the B arg and replace it with self
                let old_arg = new_sig.args.remove(possible_b_args[0]);    
                new_sig.args.insert(0, FnArg{
                    arg_name: None,
                    arg_type: quote! { &self }
                });
                let sig_tokens = render_fn_signature(new_sig)?;
                l2_impls.extend(sig_tokens);

                //Emit an assignment, to assign self back to the original argument name
                let old_arg_name = old_arg.arg_name.clone().unwrap();
                let self_assignment_tokens = quote! {
                    let #old_arg_name = self;
                };

                l2_impls.extend(quote! {
                    {
                        #self_assignment_tokens

                        #pair_fn_body
                    }
                });
            }

            true
        } else {
            false
        }
    } else {
        false
    };

    if !found_pair {
        //Emit methods with an "unimplemented" body
        for orig_fn_name in fn_sigs.keys() {

            //Get the tokens for the l2 fn signature from the l2_sigs HashMap, and prepend a '_' to the arg names
            // in order to supress "unused variable" warnings
            let (l2_sig, _l2_sig_tokens) = l2_sigs.get(&(orig_fn_name, a_type_name)).unwrap();
            let mut new_sig = l2_sig.clone();
            for arg in new_sig.args.iter_mut() {
                if let Some(arg_name) = &mut arg.arg_name {
                    *arg_name = Ident::new(&format!("_{}", arg_name.to_string()), arg_name.span());
                }
            }
            let new_sig_tokens = render_fn_signature(new_sig)?;

            l2_impls.extend(new_sig_tokens);
            l2_impls.extend(quote! {
                {
                    unimplemented!();
                }
            });
        }
    }

    Ok(l2_impls)
}

//Replaces "#A" and "#B" placeholders with the tokens representing concrete types
fn replace_type_placeholders(input_stream: TokenStream, type_a: &TokenStream, type_b: &TokenStream) -> Result<TokenStream, SyntaxError> {

    let mut fn_body_iter = input_stream.into_iter();
    let mut previous_hash = false;
    recursive_scan(&mut fn_body_iter, &mut |token, stream| {

        if previous_hash {
            if let TokenTree::Ident(ident) = token {
                match ident.to_string().as_str() {
                    "A" => {
                        stream.extend([type_a.clone()]);
                    },
                    "B" => {
                        stream.extend([type_b.clone()]);
                    },
                    _ => return Err(format!("unknown type macro identifier, #{}", ident.to_string())),
                };
                previous_hash = false;
                return Ok(());
            } else {
                return Err(format!("expected special type macro identifier"));
            }
        }

        if let TokenTree::Punct(punct) = &token {
            if punct.as_char() == '#' {
                previous_hash = true;
                return Ok(());
            }
        }

        stream.extend([token]);
        Ok(())
    })
}

fn tokens_to_string(tokens: TokenStream) -> String {
    let mut out_string = "".to_string();
    for token in tokens.into_iter() {
        match token {
            TokenTree::Ident(ident) => {
                out_string.push_str(&ident.to_string());
            }
            TokenTree::Literal(literal) => {
                out_string.push_str(&literal.to_string());
            }
            TokenTree::Punct(punct) => {
                let punct_str = match punct.as_char() {
                    '&' => "_amp_",
                    '*' => "_star_",
                    '.' => "_dot_",
                    ',' => "_comma_",
                    '#' => "_hash_",
                    '@' => "_at_",
                    '!' => "_bang_",
                    '$' => "_dollar_",
                    '%' => "_pct_",
                    '^' => "_caret_",
                    '<' => "_lt_",
                    '>' => "_gt_",
                    _ => "_punct_"
                };
                out_string.push_str(punct_str);
            }
            TokenTree::Group(group) => {

                let (open_delim, close_delim) = match group.delimiter() {
                    Delimiter::Brace => ("_open_curly_", "_close_curly_"),
                    Delimiter::Parenthesis => ("_open_paren_", "_close_paren_"),
                    Delimiter::Bracket => ("_open_square_", "_close_square_"),
                    Delimiter::None => ("_open_none_", "_close_none_"),
                };
                let insides = tokens_to_string(group.stream());

                out_string.push_str(open_delim);
                out_string.push_str(&insides);
                out_string.push_str(close_delim);
            }

        }
    }
    out_string
}

fn render_fn_signature(sig: FnSignature) -> Result<TokenStream, SyntaxError> {

    let fn_name = sig.fn_name;

    let generic_tokens = if !sig.generics.is_empty() {
        let sig_generics = sig.generics;
        quote! {
            < #sig_generics >
        }
    } else {
        TokenStream::new()
    };

    let mut arg_list_tokens = TokenStream::new();
    for arg in sig.args {
        if let Some(arg_name_ident) = arg.arg_name {
            arg_list_tokens.extend([TokenTree::Ident(arg_name_ident), TokenTree::Punct(Punct::new(':', Spacing::Alone))]);
        }

        arg_list_tokens.extend(arg.arg_type);
        arg_list_tokens.extend([TokenTree::Punct(Punct::new(',', Spacing::Alone))]);
    }
    let result_tokens = if !sig.result.is_empty() {
        let sig_results = sig.result;
        quote! {
            -> #sig_results
        }
    } else {
        TokenStream::new()
    };

    let pub_qualifiers = sig.pub_qualifiers;

    let sig_tokens = quote! {
        #pub_qualifiers fn #fn_name #generic_tokens (#arg_list_tokens) #result_tokens
    };

    Ok(sig_tokens)
}


// // // // // // // // // // // // // // // // // // // // // // // // // // // // // // // // // // //
// Reference (example of input and the corresponding output, in a form that's easier to read)
// // // // // // // // // // // // // // // // // // // // // // // // // // // // // // // // // // //

//Update: Just run `cargo expand`

//=====================================================================================
// Tests
//=====================================================================================

//Positive Examples:
// i32
// val: i32
// a: &dyn PrimInt
// &i32
// &Vec<&i32>
// Box<dyn PrimInt>
// HashMap<String, Box<dyn PrimInt>>
//
//Negative Examples:
// NULL (no tokens)
// val:
// HashMap<String
//
#[test]
fn require_fn_arg_test() {

    //Positive Examples:
    let mut input_tokens_iter = quote! {
        i32
    }.into_iter();
    assert!(require_fn_arg(&mut input_tokens_iter, Span::call_site()).is_ok());

    let mut input_tokens_iter = quote! {
        val: i32
    }.into_iter();
    assert!(require_fn_arg(&mut input_tokens_iter, Span::call_site()).is_ok());

    let mut input_tokens_iter = quote! {
        a: &dyn PrimInt
    }.into_iter();
    assert!(require_fn_arg(&mut input_tokens_iter, Span::call_site()).is_ok());

    let mut input_tokens_iter = quote! {
        a: &i32
    }.into_iter();
    assert!(require_fn_arg(&mut input_tokens_iter, Span::call_site()).is_ok());

    let mut input_tokens_iter = quote! {
        a: &Vec<&i32>
    }.into_iter();
    assert!(require_fn_arg(&mut input_tokens_iter, Span::call_site()).is_ok());

    let mut input_tokens_iter = quote! {
        a: Box<dyn PrimInt>
    }.into_iter();
    assert!(require_fn_arg(&mut input_tokens_iter, Span::call_site()).is_ok());

    let mut input_tokens_iter = quote! {
        a: HashMap<String, Box<dyn PrimInt>>
    }.into_iter();
    assert!(require_fn_arg(&mut input_tokens_iter, Span::call_site()).is_ok());

    //Negative Examples:
    let mut input_tokens_iter = quote! {
        
    }.into_iter();
    assert!(require_fn_arg(&mut input_tokens_iter, Span::call_site()).is_err());

    let mut input_tokens_iter = quote! {
        val:
    }.into_iter();
    assert!(require_fn_arg(&mut input_tokens_iter, Span::call_site()).is_err());

    let mut input_tokens_iter = quote! {
        HashMap<String //Ugg.  This bad syntax screws up my text editor's pretty printer, but the compiler is fine
    }.into_iter();
    assert!(require_fn_arg(&mut input_tokens_iter, Span::call_site()).is_err());

}

#[test]
fn require_fn_signature_test() {

    use quote::{quote};
    use crate::parse::require_fn_signature;     

    //=====================================================================================
    //Test that I can parse a basic signature
    let mut input_tokens_iter = quote! {
        fn min_max(val: i32, min: &i32, max: &i32);
    }.into_iter();

    let result_signature = require_fn_signature(&mut input_tokens_iter, true, Span::call_site()).unwrap();

    assert_eq!(result_signature.fn_name, "min_max");

    //=====================================================================================
    //Next test that I can parse "pub"
    let mut input_tokens_iter = quote! {
        pub fn min_max(val: i32, min: &i32, max: &i32) -> Result<i32, String>;
    }.into_iter();

    let result_signature = require_fn_signature(&mut input_tokens_iter, true, Span::call_site()).unwrap();

    let mut pub_qualifiers_iter = result_signature.pub_qualifiers.into_iter();
    require_keyword(&mut pub_qualifiers_iter, "pub", Span::call_site()).unwrap();
    assert!(pub_qualifiers_iter.next().is_none());

    //=====================================================================================
    //Next test that I can parse pub(crate)
    let mut input_tokens_iter = quote! {
        pub(crate) fn min_max(val: i32, min: &i32, max: &i32) -> Result<i32, String>;
    }.into_iter();

    let result_signature = require_fn_signature(&mut input_tokens_iter, true, Span::call_site()).unwrap();

    let mut pub_qualifiers_iter = result_signature.pub_qualifiers.into_iter();
    require_keyword(&mut pub_qualifiers_iter, "pub", Span::call_site()).unwrap();
    require_group(&mut pub_qualifiers_iter, Delimiter::Parenthesis, Span::call_site(), "missing '(crate)'").unwrap();
    assert!(pub_qualifiers_iter.next().is_none());

    //=====================================================================================
    //Next, test that I can parse some simple generics
    let mut input_tokens_iter = quote! {
        fn min_max<A, B>(val: i32, min: &A, max: &B) -> Result<A, String>;
    }.into_iter();

    let result_signature = require_fn_signature(&mut input_tokens_iter, true, Span::call_site()).unwrap();

    let mut generics_iter = result_signature.generics.into_iter();
    let _ = require_ident(&mut generics_iter, Span::call_site()).unwrap();
    let _ = require_punct(&mut generics_iter, ',', Span::call_site()).unwrap();
    let _ = require_ident(&mut generics_iter, Span::call_site()).unwrap();
    assert!(generics_iter.next().is_none());

    //=====================================================================================
    //Next, test that I can handle complicated nested generics
    let mut input_tokens_iter = quote! {
        fn min_max<A:From<i32>, B>(val: i32, min: &A, max: &B) -> Result<A, String>;
    }.into_iter();

    let result_signature = require_fn_signature(&mut input_tokens_iter, true, Span::call_site()).unwrap();

    let mut generics_iter = result_signature.generics.into_iter();
    let _ = require_ident(&mut generics_iter, Span::call_site()).unwrap();
    let _ = require_punct(&mut generics_iter, ':', Span::call_site()).unwrap();
    let _ = require_ident(&mut generics_iter, Span::call_site()).unwrap();
    let _ = require_angle_group(&mut generics_iter, Span::call_site(), "expecting angle brackets").unwrap();
    let _ = require_punct(&mut generics_iter, ',', Span::call_site()).unwrap();
    let _ = require_ident(&mut generics_iter, Span::call_site()).unwrap();
    assert!(generics_iter.next().is_none());

    //=====================================================================================
    //Next, test that I get all the args with names
    let mut input_tokens_iter = quote! {
        fn min_max(val: i32, min: &i32, max: &i32);
    }.into_iter();

    let result_signature = require_fn_signature(&mut input_tokens_iter, true, Span::call_site()).unwrap();

    assert_eq!(result_signature.args.len(), 3);
    assert!(result_signature.args[0].arg_name.is_some());
    let mut arg2_type_iter = result_signature.args[2].arg_type.clone().into_iter();
    let _ = require_punct(&mut arg2_type_iter, '&', Span::call_site()).unwrap();
    let _ = require_ident(&mut arg2_type_iter, Span::call_site()).unwrap();
    
    //=====================================================================================
    //Next, test that I get all the args without names
    let mut input_tokens_iter = quote! {
        fn min_max(i32, &i32, &i32);
    }.into_iter();

    let result_signature = require_fn_signature(&mut input_tokens_iter, true, Span::call_site()).unwrap();

    assert_eq!(result_signature.args.len(), 3);
    assert!(result_signature.args[0].arg_name.is_none());
    let mut arg2_type_iter = result_signature.args[2].arg_type.clone().into_iter();
    let _ = require_punct(&mut arg2_type_iter, '&', Span::call_site()).unwrap();
    let _ = require_ident(&mut arg2_type_iter, Span::call_site()).unwrap();

    //=====================================================================================
    //Next, test that I can handle no arguments
    let mut input_tokens_iter = quote! {
        fn min_max();
    }.into_iter();

    require_fn_signature(&mut input_tokens_iter, true, Span::call_site()).unwrap();

    //=====================================================================================
    //Next, test that I can parse a result
    let mut input_tokens_iter = quote! {
        fn min_max<A>() -> Result<A, String>;
    }.into_iter();

    let result_signature = require_fn_signature(&mut input_tokens_iter, true, Span::call_site()).unwrap();

    let mut result_iter = result_signature.result.into_iter();
    let _ = require_ident(&mut result_iter, Span::call_site()).unwrap();
    let _ = require_angle_group(&mut result_iter, Span::call_site(), "expecting angle brackets").unwrap();
}
