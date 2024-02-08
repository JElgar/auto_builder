use auto_builder::Builder;

#[test]
fn creates_builder() {
    #[derive(Builder)]
    struct A {
        x: i32,
        y: i32,
        z: String,
    }

    let a = A::builder().set_x(1).set_y(2).set_z("abc").build();
    assert_eq!(a.x, 1);
    assert_eq!(a.y, 2);
    assert_eq!(a.z, String::from("abc"));
}

#[test]
fn build_without_optional() {
    #[derive(Builder)]
    struct A {
        x: i32,
        y: Option<i32>,
        z: String,
    }

    let a = A::builder().set_x(1).set_z("abc").build();
    assert_eq!(a.x, 1);
    assert_eq!(a.y, None);
    assert_eq!(a.z, String::from("abc"));
}

#[test]
fn build_with_optional() {
    #[derive(Builder)]
    struct A {
        x: i32,
        y: Option<i32>,
        z: String,
    }

    let a = A::builder().set_x(1).set_y(2).set_z("abc").build();
    assert_eq!(a.x, 1);
    assert_eq!(a.y, Some(2));
    assert_eq!(a.z, String::from("abc"));
}

#[test]
fn build_with_into() {
    struct P {
        x: i32,
    }

    impl From<i32> for P {
        fn from(x: i32) -> Self {
            P { x }
        }
    }

    #[derive(Builder)]
    struct A {
        x: i32,
        y: P,
        z: String,
    }

    let a = A::builder().set_x(1).set_y(2).set_z("abc").build();
    assert_eq!(a.x, 1);
    assert_eq!(a.y.x, 2);
    assert_eq!(a.z, String::from("abc"));
}

#[test]
fn build_with_default() {
    struct A {
        x: i32,
        y: String,
    }

    impl Default for A {
        fn default() -> Self {
            A {
                x: 10,
                y: String::from("Hello"),
            }
        }
    }

    #[derive(Builder)]
    struct B {
        #[auto_builder(default)]
        a: A,
        b: String,
    }

    let b = B::builder().set_b("world").build();
    assert_eq!(b.a.x, 10);
    assert_eq!(b.a.y, String::from("Hello"));
    assert_eq!(b.b, String::from("world"));
}
