

//I want to support:
//1. an arbitrary function name and signiture
//2. both stand-alone funtions as well as methods
//3. Functions where both dyns are of the same type as well as functions where A and B have different types
//4. Adding additional pairs across multiple impl blocks
//
//5. Multiple function within the same block?
//6. Test that it works with and without pub

//Thoughts: I probably need to make the top-level fn declaration be one macro, and the impls
// be another.  Therefore there should be a symbol connecting them.

//The problem is that somehow I need to know what the arguments are in the impl block
// Is it possible that I can just use the function / method names??

//=====================================================================================
//double_dyn_fn manual expansion
//=====================================================================================

//This macro generates:
// the top-level fn that calls into the min.ddyn_NumTrait_level_one_min_max_fn(val, max);
// the trait definition for ddyn_NumTrait_level_one_min_max

// Alternately, instead of the double_dyn_fn! macro, we might want something that intercepts the impl for a
// a method on A.  So it emits all 

//For example, 
/*

trait NumTrait {
    fn some_func(&self, other: &dyn NumTrait);
}

impl NumTrait for i32 {

    fn some_func(&self, other: &dyn NumTrait) {
        other.ddyn_ATrait_i32_level_two_some_func(self);
    }
}
*/

// double_dyn_fn!{
//     fn min_max(val: i32, min: &dyn #A, max: &dyn #B) -> Result<i32, String>;
// }

//=====================================================================================
//double_dyn_fn manual expansion
//=====================================================================================

fn min_max(val: i32, min: &dyn Ddyn_LevelOne_min_max, max: &dyn Ddyn_LevelTwo_min_max) -> Result<i32, String> {
    min.ddyn_level_one_min_max_fn(val, max)
}

//=====================================================================================
//double_dyn_impl test
//=====================================================================================


//This macro generates:
// For every ATYPE, a trait definition for ddyn_NumTrait_level_two_min_max_ATYPE, e.g. ddyn_NumTrait_level_two_min_max_i32
// For every AType, the trait impl for ddyn_NumTrait_level_one_min_max, for every A, that calls max.ddyn_NumTrait_level_two_min_max_i32(val, min:i32);
// For every AB Combo, a trait impl for ddyn_NumTrait_level_two_min_max_i32

// double_dyn_impl!{

//     (i32, i32)
//     {
//         fn min_max(val: i32, min: &i32, max: &i32) -> Result<i32, String> {
//             Ok(2)
//         }
//     }

//     (f32, f32)
//     {
//         fn min_max(val: i32, min: &f32, max: &f32) -> Result<i32, String> {
//             Ok(3)
//         }
//     }

//     #[commutative]
//     (i32, f32)
//     {
//         fn min_max(val: i32, min: &i32, max: &f32) -> Result<i32, String> {
//             Ok(4)
//         }
//     }
// }

//=====================================================================================
//double_dyn_impl manual expansion
//=====================================================================================

trait Ddyn_LevelOne_min_max {
    fn ddyn_level_one_min_max_fn(&self, val: i32, max: &dyn Ddyn_LevelTwo_min_max) -> Result<i32, String>;
}

trait Ddyn_LevelTwo_min_max {
    //A function for each B type
    fn ddyn_level_two_min_max_i32_fn(&self, val: i32, min: &i32) -> Result<i32, String>;
    fn ddyn_level_two_min_max_f32_fn(&self, val: i32, min: &f32) -> Result<i32, String>;
}

impl Ddyn_LevelOne_min_max for i32 {
    fn ddyn_level_one_min_max_fn(&self, val: i32, max: &dyn Ddyn_LevelTwo_min_max) -> Result<i32, String> {
        max.ddyn_level_two_min_max_i32_fn(val, self)
    }
}

impl Ddyn_LevelOne_min_max for f32 {
    fn ddyn_level_one_min_max_fn(&self, val: i32, max: &dyn Ddyn_LevelTwo_min_max) -> Result<i32, String> {
        max.ddyn_level_two_min_max_f32_fn(val, self)
    }
}

impl Ddyn_LevelTwo_min_max for i32 {
    fn ddyn_level_two_min_max_i32_fn(&self, val: i32, min: &i32) -> Result<i32, String> {
        Ok(2)
    }
    fn ddyn_level_two_min_max_f32_fn(&self, val: i32, min: &f32) -> Result<i32, String> {
        Ok(4)
    }
}

impl Ddyn_LevelTwo_min_max for f32 {
    fn ddyn_level_two_min_max_i32_fn(&self, val: i32, min: &i32) -> Result<i32, String> {
        Ok(4)
    }
    fn ddyn_level_two_min_max_f32_fn(&self, val: i32, min: &f32) -> Result<i32, String> {
        Ok(3)
    }
}

//=====================================================================================
// main
//=====================================================================================

fn main() {


    let val = min_max(5, &2.0, &5.0).unwrap();
    println!("{}", val);
}
