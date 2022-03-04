
use double_dyn_macros::double_dyn_fn;

//I want to support:
//√1. an arbitrary function name and signiture
//2. both stand-alone funtions as well as methods
//√3. Functions where both dyns are of the same type as well as functions where A and B have different types
//4. Adding additional pairs across multiple impl blocks
//√5. Multiple function within the same block?
//6. Test that it works with and without pub


//=====================================================================================
// main
//=====================================================================================

fn main() {

}

/*

Is there some reason that this won't work for a use case that you'd like to see it to work for?

I did my best to catch as many errors as I could envision and provide reasonable error messages.  But I may have missed some.




Unfortunately this isn't nearly as powerful as I'd like it to be.  Specifically, all of the
permutations need to be defined in one block, which feels pretty limiting.

Does anyone have a good work around for the lack of https://github.com/rust-lang/rust/issues/44034 


//TODO, have a test case for passing a third dyn trait arg that isn't either A or B

//TODO, write a blurb about how args are identified

//Mention trait bounding, for created traits

//Explain fundamental limits around generics, and explain the possible exception for B-type implementations

Tags:
"Multifunction?" "dyn" "dispatch" "dynamic" "object" "oriented"

Limitations
One Block
Can't use within existing trait
Visibility qualifiers (e.g. 'pub') must be the same for all functions in the block
Args must be `&dyn MyTrait`, as opposed to `Box<dyn MyTrait>`
`where` clauses aren't supported.
some generics won't work

Some errors and warnings will be reported multiple times

Thanks
Thanks to [@dtolnay](https://github.com/dtolnay) and [@h2co3](http://h2co3.github.io/)

*/

