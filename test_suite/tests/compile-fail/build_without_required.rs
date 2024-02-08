use auto_builder::Builder;

fn main() {
    #[derive(Builder)]
    struct A {
        x: i32,
        y: i32,
        z: String,
    }

    let a = A::builder().set_x(1).set_y(2).build();
}
