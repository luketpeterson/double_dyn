
use double_dyn_macros::double_dyn_fn;

//I want to support:
//√1. an arbitrary function name and signiture
//2. both stand-alone funtions as well as methods
//√3. Functions where both dyns are of the same type as well as functions where A and B have different types
//4. Adding additional pairs across multiple impl blocks
//√5. Multiple function within the same block?
//6. Test that it works with and without pub

double_dyn_fn!{
    type A: MyTraitA;
    type B: MyTraitB: std::fmt::Display;

    fn min_max(val: i32, min: &dyn MyTraitA, max: &dyn MyTraitB) -> Result<i32, String>;
    fn multiply(a: &dyn MyTraitA, b: &dyn MyTraitB) -> Box<dyn MyTraitB>;

    impl for <i32, String>
    {
        fn min_max(val: i32, min: &i32, max: &String) -> Result<i32, String> {
            let max_as_int = max.parse::<i32>().unwrap();

            if val < *min {Ok(*min)} else
            if val > max_as_int {Ok(max_as_int)} else
            {Ok(val)}
        }

        fn multiply(a: &i32, b: &String) -> Box<dyn MyTraitB> {
            let multiplied_val = *a * b.parse::<i32>().unwrap();
            Box::new(multiplied_val.to_string())
        }
    }

    impl for <i32, f32>
    {
        fn min_max(val: i32, min: &#A, max: &#B) -> Result<i32, String> {
            if (val as #A) < *min {Ok(*min as i32)} else
            if (val as #B) > *max {Ok(*max as i32)} else
            {Ok(val)}
        }

        fn multiply(a: &#A, b: &#B) -> Box<dyn MyTraitB> {
            Box::new((*a as #B) * *b)
        }
    }

}

// double_dyn_fn!{
//     type A: MyTrait;
//     type B: MyTrait;

//     fn min_max(val: i32, min: &dyn MyTrait, max: &dyn MyTrait) -> Result<i32, String>;
//     fn multiply(min: &dyn MyTrait, max: &dyn MyTrait) -> Box<dyn MyTrait>;

//     impl for <i32, i32>
//     {
//         fn min_max(val: i32, min: &i32, max: &i32) -> Result<i32, String> {
//             if val < *min {Ok(*min)} else
//             if val > *max {Ok(*max)} else
//             {Ok(val)}
//         }

//         fn multiply(same_min: &i32, same_max: &i32) -> Box<dyn MyTrait> {
//             Box::new(*same_min * *same_max)
//         }
//     }

//     #[commutative]
//     impl for <i32, f32>
//     {
//         fn min_max(val: i32, com_min: &#A, com_max: &#B) -> Result<i32, String> {
//             if (val as #A) < *com_min {Ok(*com_min as i32)} else
//             if (val as #B) > *com_max {Ok(*com_max as i32)} else
//             {Ok(val)}
//         }

//         fn multiply(min: &#A, max: &#B) -> Box<dyn MyTrait> {
//             Box::new((*min as #B) * *max)
//         }
//     }
// }

//=====================================================================================
// main
//=====================================================================================

fn main() {

    // //Reciprocal Tests
    // let val = min_max(5, &2, &7).unwrap();
    // println!("{}", val);

    // let val = min_max(5, &2, &7.0).unwrap();
    // println!("{}", val);

    // let val = min_max(5, &2.0, &7).unwrap();
    // println!("{}", val);

    // let val = min_max(5, &2.0, &7.0).unwrap();
    // println!("{}", val);

    //Separate tests
    let val = min_max(5, &2, &"7".to_string()).unwrap();
    println!("{}", val);

    let val = multiply(&2, &"7".to_string());
    println!("{}", val);

}

/*

Is there some reason that this won't work for a use case that you'd like to see it to work for?

I did my best to catch as many errors as I could envision and provide reasonable error messages.  But I may have missed some.




Unfortunately this isn't nearly as powerful as I'd like it to be.  Specifically, all of the
permutations need to be defined in one block, which feels pretty limiting.

Does anyone have a good work around for the lack of https://github.com/rust-lang/rust/issues/44034 


//TODO, have a test case for passing a third dyn trait arg that isn't either A or B

//TODO, write a blurb about how args are identified

Tags:
"Multifunction?" "dyn" "dispatch" "dynamic"

Limitations
One Block
Can't use within existing trait
Visibility qualifiers (e.g. 'pub') must be the same for all functions in the block
Args must be `&dyn MyTrait`, as opposed to `Box<dyn MyTrait>`
`where` clauses aren't supported.
some generics won't work

Thanks
Thanks to [@dtolnay](https://github.com/dtolnay) and [@h2co3](http://h2co3.github.io/)

*/

