
use core::any::{Any, TypeId};
use std::collections::{HashMap};
use std::time::{Duration, SystemTime};

struct Registry {
    map: HashMap<(TypeId, TypeId), fn(&i32, &i32)>
}

impl Registry {

    fn new() -> Self {
        Self {
            map: HashMap::new()
        }
    }

    fn register_func<A: 'static, B: 'static>(&mut self, f: fn(&A, &B)) {

        let type_id_a = TypeId::of::<A>();
        let type_id_b = TypeId::of::<B>();
        
        let cast_f = unsafe{ std::mem::transmute(f) };
        self.map.insert((type_id_a, type_id_b), cast_f);
    }

    fn call_func(&self, a: &dyn Any, b: &dyn Any) {

        //Retrieve the function pointer
        let type_id_a = a.type_id();
        let type_id_b = b.type_id();
        
        let generic_f = self.map.get(&(type_id_a, type_id_b)).unwrap();

        let cast_a = unsafe{ &*(a as *const dyn Any as *const i32) };
        let cast_b = unsafe{ &*(b as *const dyn Any as *const i32) };
        generic_f(cast_a, cast_b);
    }
}

fn pair_func_a(a: &i32, b: &i32) {
    println!("func_a({}, {})", a, b);
}

fn pair_func_b(a: &i32, b: &char) {
    println!("func_b({}, {})", a, b);
}

fn pair_func_c(a: &char, b: &i32) {
    println!("func_c({}, {})", a, b);
}

fn main() {

    let mut resistry = Registry::new();

    resistry.register_func(pair_func_a);
    resistry.register_func(pair_func_b);
    resistry.register_func(pair_func_c);

    resistry.call_func(&5, &1);
    resistry.call_func(&5, &'b');
    resistry.call_func(&'c', &1);

    static_baseline();
    dyn_baseline();
    double_dyn_time();
}

//-----------------------------------------
//Benchmarks
//-----------------------------------------

fn static_func(a: &i32, b: &i32) {
    //Just to prevent the optimizer from eliminating the call
    if *a == 99999 {
        println!("single_static_a({}, {})", a, b);
    }
}

fn static_baseline() {

    let timer_start = SystemTime::now();

    for i in 0..1000000 {
        static_func(&i, &i);
    }

    println!("Static Baseline Time = {:?}", timer_start.elapsed().unwrap_or(Duration::new(0, 0))); 
}

trait SingleDyn : Any {
    fn dyn_func(&self, b: &i32);
}

impl SingleDyn for i32 {
    fn dyn_func(&self, b: &i32) {
        //Just to prevent the optimizer from eliminating the call
        if *self == 99999 {
            println!("single_dyn_a({}, {})", self, b);
        }
    }
}

fn single_dyn_dispatch(a: &dyn SingleDyn, b: &i32) {
    a.dyn_func(b); //I suspect the dyn dispatch might be being optimized away :-(
}

fn dyn_baseline() {

    let timer_start = SystemTime::now();

    for i in 0..1000000 {
        single_dyn_dispatch(&i, &i);
    }

    println!("Dyn Baseline Time = {:?}", timer_start.elapsed().unwrap_or(Duration::new(0, 0))); 
}

fn double_dyn_func(a: &i32, b: &i32) {
    //Just to prevent the optimizer from eliminating the call
    if *a == 99999 {
        println!("double_dyn_a({}, {})", a, b);
    }
}

fn double_dyn_time() {

    let mut resistry = Registry::new();
    resistry.register_func(double_dyn_func);
    
    let timer_start = SystemTime::now();

    for i in 0..1000000 {
        resistry.call_func(&i, &i);
    }

    println!("Double Dyn Time = {:?}", timer_start.elapsed().unwrap_or(Duration::new(0, 0))); 
}
