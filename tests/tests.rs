
use double_dyn::double_dyn;

//=====================================================================================
// Top-Level Tests
//=====================================================================================

#[test]
fn separate_traits_test() {

    double_dyn!{
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

    let val = min_max(5, &2, &"7".to_string()).unwrap();
    assert_eq!(val, 5);

    let val = multiply(&2, &"7".to_string());
    assert_eq!(format!("{}", val), "14");

    let val = multiply(&2, &7.5);
    assert_eq!(format!("{}", val), "15");
}

#[test]
fn one_trait_test() {

    double_dyn!{
        type A: MyTrait: std::fmt::Display;
        type B: MyTrait;

        fn min_max(val: i32, min: &dyn MyTrait, max: &dyn MyTrait) -> Result<i32, String>;
        fn multiply(a: &dyn MyTrait, b: &dyn MyTrait) -> Box<dyn MyTrait>;

        impl for <i32, i32>
        {
            fn min_max(val: i32, min: &i32, max: &i32) -> Result<i32, String> {
                if val < *min {Ok(*min)} else
                if val > *max {Ok(*max)} else
                {Ok(val)}
            }

            fn multiply(same_a: &i32, same_b: &i32) -> Box<dyn MyTrait> {
                Box::new(*same_a * *same_b)
            }
        }

        #[commutative]
        impl for <[i8, i16, i32, i64, i128], f32>
        {
            fn min_max(val: i32, com_min: &#A, com_max: &#B) -> Result<i32, String> {
                if (val as #A) < *com_min {Ok(*com_min as i32)} else
                if (val as #B) > *com_max {Ok(*com_max as i32)} else
                {Ok(val)}
            }

            fn multiply(a: &#A, b: &#B) -> Box<dyn MyTrait> {
                Box::new((*a as #B) * *b)
            }
        }
    }

    let val = min_max(5, &2, &7).unwrap();
    assert_eq!(val, 5);

    let val = min_max(5, &2, &7.0).unwrap();
    assert_eq!(val, 5);

    let val = min_max(5, &2.0, &7).unwrap();
    assert_eq!(val, 5);

    //This should panic because it's unimplemented in the macro implementation
    // let val = min_max(5, &2.0, &7.0).unwrap();

    let val = multiply(&2, &7.5);
    assert_eq!(format!("{}", val), "15");

    let val = multiply(&2.0, &7);
    assert_eq!(format!("{}", val), "14");

    let val = multiply(&2, &7);
    assert_eq!(format!("{}", val), "14");
}

#[test]
fn three_dyn_args_test() {

    double_dyn!{
        type A: MyTrait: std::fmt::Display;
        type B: MyTrait;

        fn multiply(other: &dyn MyTrait, a: &dyn MyTrait, b: &dyn MyTrait) -> Box<dyn MyTrait>;

        impl for <i32, i32>
        {
            fn multiply(_other: &dyn MyTrait, same_a: &i32, same_b: &i32) -> Box<dyn MyTrait> {
                Box::new(*same_a * *same_b)
            }
        }

        impl for <i32, f32>
        {
            fn multiply(_other: &dyn MyTrait, a: &i32, b: &f32) -> Box<dyn MyTrait> {
                Box::new((*a as #B) * *b)
            }
        }
    }

    let val = multiply(&0, &2, &7.5);
    assert_eq!(format!("{}", val), "15");

    let val = multiply(&0, &2, &7);
    assert_eq!(format!("{}", val), "14");
}

#[test]
fn pub_qualifier_test() {

    double_dyn!{
        type A: MyTraitA: std::fmt::Display;
        type B: MyTraitB;
    
        pub(crate) fn multiply(a: &dyn MyTraitA, b: &dyn MyTraitB) -> Box<dyn MyTraitA>;
    
        impl for <i32, i32>
        {
            fn multiply(same_a: &i32, same_b: &i32) -> Box<dyn MyTraitA> {
                Box::new(*same_a * *same_b)
            }
        }
    }
    
    let val = multiply(&2, &7);
    assert_eq!(format!("{}", val), "14");
}

#[test]
fn arg_position_inference_test() {

    double_dyn!{
        type A: MyTrait: std::fmt::Display;
        type B: MyTrait;

        fn multiply(_junk: &dyn MyTrait, a: &dyn MyTrait, _more_junk: &i32, b: &dyn MyTrait, _yet_more_junk: &i32) -> Box<dyn MyTrait>;
        
        impl for <i32, i32>
        {
            fn multiply(_junk: &dyn MyTrait, alt_name_a: &i32, _more_junk: &i32, alt_name_b: &i32, _yet_more_junk: &i32) -> Box<dyn MyTrait> {
                Box::new(*alt_name_a * *alt_name_b)
            }
        }
    }
    
    let val = multiply(&2, &3, &5, &7, &11);
    assert_eq!(format!("{}", val), "21");
}