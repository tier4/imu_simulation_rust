use nalgebra::{UnitQuaternion, Vector3};
use std::collections::VecDeque;

struct GyroInterface {
    rotation_timestamps: VecDeque<f64>,
    rotations: VecDeque<UnitQuaternion<f64>>,
    gyroscope_timestamps: VecDeque<f64>,
    angular_velocities: VecDeque<Vector3<f64>>,
}

fn binary_search(target: &VecDeque<f64>, query: f64) -> Result<usize, usize> {
    target.binary_search_by(|t| t.total_cmp(&query))
}

impl GyroInterface {
    fn new() -> Self {
        GyroInterface {
            rotation_timestamps: VecDeque::<f64>::new(),
            rotations: VecDeque::<UnitQuaternion<f64>>::new(),
            gyroscope_timestamps: VecDeque::<f64>::new(),
            angular_velocities: VecDeque::<Vector3<f64>>::new(),
        }
    }

    fn add_reference_pose(&mut self, t: f64, q: &UnitQuaternion<f64>) {
        if let Some(&back) = self.rotation_timestamps.back() {
            assert!(t > back);
        };

        self.rotation_timestamps.push_back(t);
        self.rotations.push_back(*q);
    }

    fn add_gyroscope(&mut self, t: f64, w: &Vector3<f64>) {
        if let Some(&back) = self.gyroscope_timestamps.back() {
            assert!(t > back);
        };

        self.gyroscope_timestamps.push_back(t);
        self.angular_velocities.push_back(*w);
    }

    fn get(&mut self) -> Option<(Vec<f64>, Vec<Vector3<f64>>)> {
        if self.gyroscope_timestamps.len() < 2 {
            return None;
        }

        let n = self.gyroscope_timestamps.len();
        let rt0 = self.rotation_timestamps[0];
        let rt1 = self.rotation_timestamps[1];

        println!("rt0                  = {}", rt0);
        println!("rt1                  = {}", rt1);

        if rt0 < self.gyroscope_timestamps[0] {
            return None;
        }
        if rt1 > self.gyroscope_timestamps[n - 1] {
            return None;
        }

        match binary_search(&self.gyroscope_timestamps, rt0) {
            Ok(index) => {
                self.gyroscope_timestamps.drain(..index);
                self.angular_velocities.drain(..index);
            }
            Err(index) => {
                self.gyroscope_timestamps.drain(..index - 1);
                self.angular_velocities.drain(..index - 1);
            }
        }

        self.rotations.pop_front();
        self.rotation_timestamps.pop_front();
        match binary_search(&self.gyroscope_timestamps, rt1) {
            Ok(index) => {
                let ts = self
                    .gyroscope_timestamps
                    .drain(..=index)
                    .collect::<Vec<f64>>();
                let ws = self
                    .angular_velocities
                    .drain(..=index)
                    .collect::<Vec<Vector3<f64>>>();
                return Some((ts, ws));
            }
            Err(index) => {
                let mut ts = self
                    .gyroscope_timestamps
                    .drain(..index - 1)
                    .collect::<Vec<f64>>();
                let mut ws = self
                    .angular_velocities
                    .drain(..index - 1)
                    .collect::<Vec<Vector3<f64>>>();
                let t01 = self.gyroscope_timestamps.range(0..2);
                let w01 = self.angular_velocities.range(0..2);
                ts.extend(t01);
                ws.extend(w01);
                return Some((ts, ws));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::GyroscopeGenerator;
    use nalgebra::Quaternion;

    const PI: f64 = std::f64::consts::PI;

    #[test]
    fn test_binary_search() {
        let mut v = VecDeque::new();
        v.extend(&[0.2, 0.4, 0.6, 0.8]);
        assert_eq!(binary_search(&v, 0.1), Err(0));
        assert_eq!(binary_search(&v, 0.5), Err(2));
        assert_eq!(binary_search(&v, 0.8), Ok(3));
    }

    fn quat(t: f64) -> UnitQuaternion<f64> {
        let x = f64::sin(2. * PI * 1. * t);
        let y = f64::sin(2. * PI * 2. * t);
        let z = f64::sin(2. * PI * 3. * t);
        let w2 = 1.0 - (1. / 3.) * (x * x + y * y + z * z);
        assert!(w2 > 0.0);
        let w = f64::sqrt(w2);

        UnitQuaternion::new_normalize(Quaternion::new(w, x, y, z))
    }

    fn time(i: usize) -> f64 {
        i as f64
    }

    #[test]
    fn test_get_without_time_offset() {
        let mut interface = GyroInterface::new();

        let generator = GyroscopeGenerator::new(time, quat);

        let (t, omega) = generator.angular_velocity(4);
        interface.add_gyroscope(t, &omega);

        let (ta, qa) = generator.rotation(4);
        interface.add_reference_pose(ta, &qa);

        let (t, omega) = generator.angular_velocity(5);
        interface.add_gyroscope(t, &omega);

        let (t, omega) = generator.angular_velocity(6);
        interface.add_gyroscope(t, &omega);

        let (t, omega) = generator.angular_velocity(7);
        interface.add_gyroscope(t, &omega);

        let (tb, qb) = generator.rotation(7);
        interface.add_reference_pose(tb, &qb);

        let (t, omega) = generator.angular_velocity(8);
        interface.add_gyroscope(t, &omega);

        match interface.get() {
            Some((ts, _ws)) => assert_eq!(ts, [4., 5., 6., 7.]),
            None => assert!(false),
        }
    }

    #[test]
    fn test_get_start_reference_time_too_early() {
        let mut interface = GyroInterface::new();

        let generator = GyroscopeGenerator::new(time, quat);

        let (ta, qa) = generator.rotation(4);
        interface.add_reference_pose(ta, &qa);

        let (t, omega) = generator.angular_velocity(5);
        interface.add_gyroscope(t, &omega);

        let (t, omega) = generator.angular_velocity(6);
        interface.add_gyroscope(t, &omega);

        let (t, omega) = generator.angular_velocity(7);
        interface.add_gyroscope(t, &omega);

        let (tb, qb) = generator.rotation(7);
        interface.add_reference_pose(tb, &qb);

        let (t, omega) = generator.angular_velocity(8);
        interface.add_gyroscope(t, &omega);

        let (t, omega) = generator.angular_velocity(9);
        interface.add_gyroscope(t, &omega);

        assert_eq!(interface.get(), None);
    }

    #[test]
    fn test_get_end_reference_time_too_late() {
        let mut interface = GyroInterface::new();

        let generator = GyroscopeGenerator::new(time, quat);

        let (t, omega) = generator.angular_velocity(3);
        interface.add_gyroscope(t, &omega);

        let (t, omega) = generator.angular_velocity(4);
        interface.add_gyroscope(t, &omega);

        let (ta, qa) = generator.rotation(4);
        interface.add_reference_pose(ta, &qa);

        let (t, omega) = generator.angular_velocity(5);
        interface.add_gyroscope(t, &omega);

        let (t, omega) = generator.angular_velocity(6);
        interface.add_gyroscope(t, &omega);

        let (tb, qb) = generator.rotation(7);
        interface.add_reference_pose(tb, &qb);

        assert_eq!(interface.get(), None);
    }

    #[test]
    fn test_get_start_timestamp_matches() {
        let mut interface = GyroInterface::new();

        let generator = GyroscopeGenerator::new(time, quat);

        let (t, omega) = generator.angular_velocity(5);
        interface.add_gyroscope(t, &omega);

        let (ta, qa) = generator.rotation(5);
        interface.add_reference_pose(ta, &qa);

        let (t, omega) = generator.angular_velocity(6);
        interface.add_gyroscope(t, &omega);

        let (t, omega) = generator.angular_velocity(7);
        interface.add_gyroscope(t, &omega);

        let (tb, qb) = generator.rotation(7);
        interface.add_reference_pose(tb, &qb);

        let (t, omega) = generator.angular_velocity(8);
        interface.add_gyroscope(t, &omega);

        let (t, omega) = generator.angular_velocity(9);
        interface.add_gyroscope(t, &omega);

        match interface.get() {
            Some((ts, _ws)) => assert_eq!(ts, [5., 6., 7.]),
            None => assert!(false),
        }
    }

    #[test]
    fn test_get_with_both_time_offsets() {
        let mut interface = GyroInterface::new();

        let generator = GyroscopeGenerator::new(time, quat);

        let (t, omega) = generator.angular_velocity(2);
        interface.add_gyroscope(t, &omega);

        let (t, omega) = generator.angular_velocity(4);
        interface.add_gyroscope(t, &omega);

        let (ta, qa) = generator.rotation(5);
        interface.add_reference_pose(ta, &qa);

        let (t, omega) = generator.angular_velocity(6);
        interface.add_gyroscope(t, &omega);

        let (t, omega) = generator.angular_velocity(8);
        interface.add_gyroscope(t, &omega);

        let (tb, qb) = generator.rotation(9);
        interface.add_reference_pose(tb, &qb);

        let (t, omega) = generator.angular_velocity(10);
        interface.add_gyroscope(t, &omega);

        let (t, omega) = generator.angular_velocity(12);
        interface.add_gyroscope(t, &omega);

        match interface.get() {
            Some((ts, _ws)) => assert_eq!(ts, [4., 6., 8., 10.]),
            None => assert!(false),
        }
    }

    #[test]
    fn test_get_only_start_time_offset() {
        let mut interface = GyroInterface::new();

        let generator = GyroscopeGenerator::new(time, quat);

        let (t, omega) = generator.angular_velocity(2);
        interface.add_gyroscope(t, &omega);

        let (t, omega) = generator.angular_velocity(4);
        interface.add_gyroscope(t, &omega);

        let (ta, qa) = generator.rotation(5);
        interface.add_reference_pose(ta, &qa);

        let (t, omega) = generator.angular_velocity(6);
        interface.add_gyroscope(t, &omega);

        let (t, omega) = generator.angular_velocity(8);
        interface.add_gyroscope(t, &omega);

        let (t, omega) = generator.angular_velocity(10);
        interface.add_gyroscope(t, &omega);

        let (tb, qb) = generator.rotation(10);
        interface.add_reference_pose(tb, &qb);

        let (t, omega) = generator.angular_velocity(12);
        interface.add_gyroscope(t, &omega);

        // Include 4 to generate the interpolated angular velocity for 5
        match interface.get() {
            Some((ts, _ws)) => assert_eq!(ts, [4., 6., 8., 10.]),
            None => assert!(false),
        }
    }

    #[test]
    fn test_get_only_end_time_offset() {
        let mut interface = GyroInterface::new();

        let generator = GyroscopeGenerator::new(time, quat);

        let (t, omega) = generator.angular_velocity(2);
        interface.add_gyroscope(t, &omega);

        let (t, omega) = generator.angular_velocity(4);
        interface.add_gyroscope(t, &omega);

        let (ta, qa) = generator.rotation(4);
        interface.add_reference_pose(ta, &qa);

        let (t, omega) = generator.angular_velocity(6);
        interface.add_gyroscope(t, &omega);

        let (t, omega) = generator.angular_velocity(8);
        interface.add_gyroscope(t, &omega);

        let (tb, qb) = generator.rotation(9);
        interface.add_reference_pose(tb, &qb);

        let (t, omega) = generator.angular_velocity(10);
        interface.add_gyroscope(t, &omega);

        let (t, omega) = generator.angular_velocity(12);
        interface.add_gyroscope(t, &omega);

        // Include 10 to generate the interpolated angular velocity for 9
        match interface.get() {
            Some((ts, _ws)) => assert_eq!(ts, [4., 6., 8., 10.]),
            None => assert!(false),
        }
    }

    #[test]
    fn test_multiple_pop() {
        let mut interface = GyroInterface::new();

        let generator = GyroscopeGenerator::new(time, quat);

        let (t, omega) = generator.angular_velocity(2);
        interface.add_gyroscope(t, &omega);

        let (t, omega) = generator.angular_velocity(4);
        interface.add_gyroscope(t, &omega);

        let (ta, qa) = generator.rotation(4);
        interface.add_reference_pose(ta, &qa);

        let (t, omega) = generator.angular_velocity(6);
        interface.add_gyroscope(t, &omega);

        let (t, omega) = generator.angular_velocity(8);
        interface.add_gyroscope(t, &omega);

        let (tb, qb) = generator.rotation(9);
        interface.add_reference_pose(tb, &qb);

        let (t, omega) = generator.angular_velocity(10);
        interface.add_gyroscope(t, &omega);

        let (t, omega) = generator.angular_velocity(12);
        interface.add_gyroscope(t, &omega);

        let (tb, qb) = generator.rotation(12);
        interface.add_reference_pose(tb, &qb);

        // Include 10 to generate the interpolated angular velocity for 9
        match interface.get() {
            Some((ts, _ws)) => assert_eq!(ts, [4., 6., 8., 10.]),
            None => assert!(false),
        }

        match interface.get() {
            Some((ts, _ws)) => assert_eq!(ts, [8., 10., 12.]),
            None => assert!(false),
        }
    }
}
