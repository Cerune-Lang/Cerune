pub const EXAMPLES: &[(&str, &str)] = &[
    (
        include_str!("../../examples/recursion/factorial.ceru"),
        include_str!("../../examples/recursion/factorial.stdout"),
    ),
    (
        include_str!("../../examples/recursion/mutual.ceru"),
        include_str!("../../examples/recursion/mutual.stdout"),
    ),
    (
        include_str!("../../examples/recursion/owned_generic.ceru"),
        include_str!("../../examples/recursion/owned_generic.stdout"),
    ),
];
pub const STOPS: &[&str] = &[
    "fn again()->void{again();}print(7);again();print(99);",
    "fn first()->void{second();}fn second()->void{first();}fn main()->void{print(7);first();print(99);}",
    "fn same<T>(x:T)->T{return same::<T>(x);}print(7);print(same::<string>(\"日本語\\0\\r\\n\"));print(99);",
    "fn left<T>(x:T)->T{return right::<T>(x);}fn right<T>(x:T)->T{return left::<T>(x);}print(7);print(left::<i64>(1));print(99);",
    "fn mark(n:i64)->i64{print(n);return n;}fn again(n:i64)->i64{return again(mark(n+1));}print(again(0));print(99);",
    "fn again(n:i64)->i64{if n==1{return 1/0;}return again(n-1);}print(7);print(again(1));print(99);",
    "type Box{value:i64=make::<i64>(),marker:bool}fn make<T>()->i64{return (Box{marker:true}).value;}print(7);print(make::<i64>());print(99);",
];
