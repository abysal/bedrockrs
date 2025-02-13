use std::cmp::{max, min};
use std::io;
use std::io::Write;
use vek::Vec3;

pub fn idx_3_to_1<T: Clone + Copy + Into<usize> + vek::num_traits::AsPrimitive<usize>>(
    vec: Vec3<T>,
    height: T,
    depth: T,
) -> usize
where
    usize: From<T>,
{
    let vec = vec.as_::<usize>();
    let x = vec.x;
    let y = vec.y;
    let z = vec.z;
    let height = usize::from(height);
    let depth = usize::from(depth);

    x * depth * height + z * height + y
}
/// Checks EXCLUSIVELY if something is in range
pub fn in_range<T: PartialOrd>(min: T, max: T, val: T) -> bool {
    min < val && val < max
}

pub fn bounds_left_optimized<T: Ord + Clone>(lhs: Vec3<T>, rhs: Vec3<T>) -> (Vec3<T>, Vec3<T>) {
    let smallest_new_x = min(lhs.x.clone(), rhs.x.clone());
    let smallest_new_y = min(lhs.y.clone(), rhs.y.clone());
    let smallest_new_z = min(lhs.z.clone(), rhs.z.clone());

    let largest_new_x = max(lhs.x, rhs.x);
    let largest_new_y = max(lhs.y, rhs.y);
    let largest_new_z = max(lhs.z, rhs.z);

    (
        (smallest_new_x, smallest_new_y, smallest_new_z).into(),
        (largest_new_x, largest_new_y, largest_new_z).into(),
    )
}

#[cfg(test)]
mod test {
    use crate::utility::miner::bounds_left_optimized;
    use vek::Vec3;

    #[test]
    fn bounds_left() {
        assert_eq!(
            bounds_left_optimized(Vec3::new(-3, -3, -3), Vec3::new(5, 5, 5)),
            (Vec3::new(-3, -3, -3), Vec3::new(5, 5, 5))
        );

        assert_eq!(
            bounds_left_optimized(Vec3::new(10, -3, -3), Vec3::new(5, 5, 5)),
            (Vec3::new(5, -3, -3), Vec3::new(10, 5, 5))
        );
    }
}
