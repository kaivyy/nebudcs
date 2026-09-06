//! Tests for 1-D and 2-D performance curves.

use nebudcs_scripting::curve::{Curve1D, Curve2D, ExtrapolationMode, InterpolationMethod};

#[test]
fn test_curve_1d_linear_interpolation() {
    let curve = Curve1D {
        name: "PumpHead".into(),
        input_var: "flow".into(),
        output_var: "head".into(),
        points: vec![(0.0, 100.0), (50.0, 90.0), (100.0, 60.0)],
        interpolation: InterpolationMethod::Linear,
        extrapolation: ExtrapolationMode::Clamp,
    };
    assert!(curve.validate().is_ok());

    // Exact points
    assert!((curve.evaluate(0.0).unwrap() - 100.0).abs() < 1e-6);
    assert!((curve.evaluate(50.0).unwrap() - 90.0).abs() < 1e-6);
    assert!((curve.evaluate(100.0).unwrap() - 60.0).abs() < 1e-6);

    // Midpoints
    assert!((curve.evaluate(25.0).unwrap() - 95.0).abs() < 1e-6);
    assert!((curve.evaluate(75.0).unwrap() - 75.0).abs() < 1e-6);

    // Clamping extrapolation
    assert_eq!(curve.evaluate(-10.0).unwrap(), 100.0);
    assert_eq!(curve.evaluate(150.0).unwrap(), 60.0);
}

#[test]
fn test_curve_1d_step_interpolation() {
    let curve = Curve1D {
        name: "StageSteps".into(),
        input_var: "speed".into(),
        output_var: "stage".into(),
        points: vec![(0.0, 0.0), (600.0, 1.0), (1200.0, 2.0), (1800.0, 3.0)],
        interpolation: InterpolationMethod::Step,
        extrapolation: ExtrapolationMode::Clamp,
    };
    assert!(curve.validate().is_ok());

    assert_eq!(curve.evaluate(300.0).unwrap(), 0.0);
    assert_eq!(curve.evaluate(700.0).unwrap(), 1.0);
    assert_eq!(curve.evaluate(1500.0).unwrap(), 2.0);
}

#[test]
fn test_curve_1d_monotonic_cubic() {
    let curve = Curve1D {
        name: "SmoothValve".into(),
        input_var: "opening".into(),
        output_var: "cv".into(),
        points: vec![
            (0.0, 0.0),
            (0.25, 5.0),
            (0.5, 25.0),
            (0.75, 65.0),
            (1.0, 100.0),
        ],
        interpolation: InterpolationMethod::MonotonicCubic,
        extrapolation: ExtrapolationMode::Clamp,
    };
    assert!(curve.validate().is_ok());

    // Monotonically increasing check
    let mut prev = 0.0;
    for i in 0..=100 {
        let x = i as f64 / 100.0;
        let val = curve.evaluate(x).unwrap();
        assert!(
            val >= prev,
            "Monotonicity violated at x={x}: {val} < {prev}"
        );
        prev = val;
    }
}

#[test]
fn test_curve_1d_extrapolation_modes() {
    let mut curve = Curve1D {
        name: "TestExtrap".into(),
        input_var: "x".into(),
        output_var: "y".into(),
        points: vec![(10.0, 20.0), (20.0, 40.0)],
        interpolation: InterpolationMethod::Linear,
        extrapolation: ExtrapolationMode::Error,
    };

    assert!(curve.evaluate(5.0).is_err());
    assert!(curve.evaluate(25.0).is_err());

    curve.extrapolation = ExtrapolationMode::Linear;
    assert!((curve.evaluate(5.0).unwrap() - 10.0).abs() < 1e-6); // slope = 2.0 -> 20 - 2*5 = 10
    assert!((curve.evaluate(25.0).unwrap() - 50.0).abs() < 1e-6);
}

#[test]
fn test_curve_1d_adversarial_validation() {
    // Insufficient points
    let c_short = Curve1D::new("short", "x", "y", vec![(1.0, 2.0)]);
    assert!(c_short.validate().is_err());

    // Unsorted x points
    let c_unsorted = Curve1D::new("unsorted", "x", "y", vec![(5.0, 10.0), (2.0, 20.0)]);
    assert!(c_unsorted.validate().is_err());

    // Duplicate x points
    let c_dup = Curve1D::new("dup", "x", "y", vec![(2.0, 10.0), (2.0, 20.0)]);
    assert!(c_dup.validate().is_err());

    // Non-finite x / y
    let c_nan = Curve1D::new("nan", "x", "y", vec![(0.0, 0.0), (f64::NAN, 10.0)]);
    assert!(c_nan.validate().is_err());

    let c_inf = Curve1D::new("inf", "x", "y", vec![(0.0, 0.0), (1.0, f64::INFINITY)]);
    assert!(c_inf.validate().is_err());
}

#[test]
fn test_curve_2d_surface_bilinear_interpolation() {
    let curve2d = Curve2D {
        name: "CompressorMap".into(),
        input_x_var: "speed".into(),
        input_y_var: "flow".into(),
        output_var: "pr".into(),
        x_grid: vec![1000.0, 2000.0],
        y_grid: vec![10.0, 20.0],
        z_values: vec![
            vec![1.5, 1.2], // speed 1000: flow 10 -> 1.5, flow 20 -> 1.2
            vec![2.5, 2.0], // speed 2000: flow 10 -> 2.5, flow 20 -> 2.0
        ],
        extrapolation: ExtrapolationMode::Clamp,
    };
    assert!(curve2d.validate().is_ok());

    // Midpoint: speed 1500, flow 15 -> (1.5 + 1.2 + 2.5 + 2.0) / 4 = 1.8
    let pr = curve2d.evaluate(1500.0, 15.0).unwrap();
    assert!((pr - 1.8).abs() < 1e-6);

    // Exact corner
    assert_eq!(curve2d.evaluate(1000.0, 10.0).unwrap(), 1.5);
    assert_eq!(curve2d.evaluate(2000.0, 20.0).unwrap(), 2.0);
}
