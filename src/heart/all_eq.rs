pub struct AllEqDispatcher<T>(pub T);

pub trait AllEqViaPartialEq {
    fn all_eq(&self, other: &Self) -> bool;
}
impl<T: PartialEq> AllEqViaPartialEq for &AllEqDispatcher<&T> {
    fn all_eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

pub trait AllEqViaPtrEquality {
    fn all_eq(&self, other: &Self) -> bool;
}
impl<T> AllEqViaPtrEquality for AllEqDispatcher<&T> {
    fn all_eq(&self, other: &Self) -> bool {
        self.0 as *const T == other.0 as *const T
    }
}

#[macro_export]
macro_rules! all_eq_ {
    ($a:expr, $b:expr) => {
        (&&AllEqDispatcher($a)).all_eq(&&AllEqDispatcher($b))
    };
}
pub use all_eq_ as all_eq;

#[cfg(test)]
mod test_all_eq {
    use super::*;

    #[test]
    fn test_i32() {
        let val_a = 5i32;
        let val_b = 6i32;

        assert!(!all_eq!(&val_a, &val_b));
    }

    #[test]
    fn test_closure() {
        let clos_a = |a: i32, b: i32| {};
        assert!(all_eq!(&clos_a, &clos_a));
    }


    struct NotCopy;
    #[test]
    fn test_not_copy() {
        let not_copy_a = NotCopy;
        let not_copy_b = NotCopy;

        assert!(!all_eq!(&not_copy_a, &not_copy_b));
    }
}
