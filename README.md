
Provides a macro for implementing functions with [multiple dynamic argument dispatch](https://en.wikipedia.org/wiki/Multiple_dispatch) at runtime.

The [double_dyn_fn] macro will define the specified trait(s) and emit implementations for all of the provided types, and then emit functions that call the appropriate implementation.

# Usage

## In your Cargo.toml
I intend to publish on crates.io once I get some feedback that this crate is useful.  For now it can be pulled directly from github.

```toml
[dependencies]
double-dyn = { git = "https://github.com/luketpeterson/double-dyn" }
```
## Basics
The `double_dyn_fn` macro invocation has 3 parts.

1. Trait names for the `A` and `B` traits, along with any subtrait bounds
2. Function prototypes
3. Implementations for type pairs, in the form `<A, B>`

## Examples

```rust
use double_dyn::double_dyn_fn;

double_dyn_fn!{
    type A: MyTraitA;
    type B: MyTraitB: std::fmt::Display;

    fn multiply(a: &dyn MyTraitA, b: &dyn MyTraitB) -> Box<dyn MyTraitB>;

    impl for <i32, String>
    {
        fn multiply(a: &i32, b: &String) -> Box<dyn MyTraitB> {
            let multiplied_val = *a * b.parse::<i32>().unwrap();
            Box::new(multiplied_val.to_string())
        }
    }

    impl for <[i8, i16, i32, i64, i128], [f32, f64]>
    {
        fn multiply(a: &#A, b: &#B) -> Box<dyn MyTraitB> {
            Box::new((*a as #B) * *b)
        }
    }
}

let val = multiply(&2, &7.5);
assert_eq!(format!("{}", val), "15");
```
This macro invocation above will define the `MyTraitA` and `MyTraitB` traits, and provide implementations for all of the relevant types.

As you can see above, multiple `A` and/or `B` types may be specified in using a list in `[square brackets]`.

You may use the concrete types explicitly Within the `impl` block, or alternatively, `#A` and `#B` markers can be used as aliases within the function signature and implementation body, and they will be replaced by the type(s) they represent at compile time.

```rust
# use double_dyn::double_dyn_fn;
double_dyn_fn!{
    type A: MyTrait: std::fmt::Display;
    type B: MyTrait;

    fn multiply(a: &dyn MyTrait, b: &dyn MyTrait) -> Box<dyn MyTrait>;

    #[commutative]
    impl for <[i8, i16, i32, i64, i128], [f32, f64]>
    {
        fn multiply(a: &#A, b: &#B) -> Box<dyn MyTrait> {
            Box::new((*a as #B) * *b)
        }
    }
}

let val = multiply(&7.0, &2);
assert_eq!(format!("{}", val), "14");
```
The same trait may be supplied for both `A` and `B`.  The `A` and `B` arguments may still be of different types within the implementation, however.  The macro will attempt to infer which argument is `A` and which is `B` from the use of the `#A` or `#B` markers but will assume the first `&dyn MyTrait` argument is `A` if it is ambiguous.

The `#[commutative]` attribute will cause an additional implementation to be generated where `A` is replaced by `B` and vice-versa.

In the case where the `A` and `B` trait is the same, the bounds from the `A` trait take precedence.

You may declare multiple functions within the same `double_dyn_fn` macro invocation, and all functions will use the same trait(s).  However, every declared function must be implemented in each `impl` block.

# Limitations

- All `impls` must be in the same `double_dyn_fn` macro invocation along with the definitions.  I'd like to be able to support separating declarations from implementations and allow additional `impls` to be added as appropriate, but I don't have a robust method to communicate between each macro invocation.  This is blocked on [this issue](https://github.com/rust-lang/rust/issues/44034).

- Each `double_dyn_fn` macro invocation defines a trait or pair of traits.  This macro isn't designed to add methods to existing traits.  It is possible to use this macro to define a trait, and then make that trait a supertrait of another trait you define, thus allowing double-dyn methods on your trait.  But the lack of [trait upcasting](https://github.com/rust-lang/rust/issues/65991) in the stable compiler is still limiting.  Please contact me if you have an idea for how to make things better for adding methods to existing traits.

- Functions may not have generic arguments.  This is a fundamental limitation based on the fact that functions are transformed into trait methods, and the traits need to remain object-safe.

- `impl`s don't support generic "blanket implementations".  `A` types can never support generic types for the same reason as above; object-safety forbids generics in trait methods.  `B` types could theoretically support blanket implementations but currently the macro doesn't parse `where` clauses in the `impl`s.  Please let me know if this feature is important to you, and I can add it.

- [visibility qualifiers](https://doc.rust-lang.org/reference/visibility-and-privacy.html), e.g. `pub`, must be the same for every function prototype.  The visibility will be applied to all generated traits and functions.

- Passing owned args isn't supported.  For example, an arg must be of the form `&dyn ATrait`, as opposed to `Box<dyn ATrait>`.

- Some errors and warnings may be reported multiple times.

# Future Vision

I would like to allow the addition of new function implementations via `impl` blocks that aren't part of the original invocation.  In other words, to allow the function signatures to be in part of the code, and allow additional implementations to be added elsewhere.  Unfortunately I don't believe this is possible on account of Rust not having an ability to communicate between macro invocations.  This is discussed [here](https://github.com/rust-lang/rust/issues/44034).

I would also like to include more flexibility for implementing methods on existing traits.  See the [Limitations](#limitations) section above.  I am open to suggestions about what you would find useful.

# Acknowledgments

This crate implements a strategy proposed by [@h2co3](http://h2co3.github.io/) in [this thread](https://users.rust-lang.org/t/dyn-dispatch-on-multiple-types).

I learned how to write proc macros by studying the code of [@dtolnay](https://github.com/dtolnay), and I borrowed some utility functions from the [seq-macro](https://github.com/dtolnay/seq-macro) crate.
