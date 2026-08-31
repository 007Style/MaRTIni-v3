//! Integration tests for the propagation model.

use martini::sim::PropagationModel;
use martini::config::RadioConfig;

fn default_radio() -> RadioConfig {
    RadioConfig::default()
}

/// path_loss_db increases with distance.
#[test]
fn path_loss_increases_with_distance() {
    let radio = default_radio();
    let pl_100 = PropagationModel::path_loss_db(100.0, &radio, 0.0);
    let pl_500 = PropagationModel::path_loss_db(500.0, &radio, 0.0);
    let pl_1000 = PropagationModel::path_loss_db(1000.0, &radio, 0.0);
    assert!(pl_100 < pl_500, "PL(100m)={} should be < PL(500m)={}", pl_100, pl_500);
    assert!(pl_500 < pl_1000, "PL(500m)={} should be < PL(1000m)={}", pl_500, pl_1000);
}

/// sinr_db decreases as number of interferers increases.
#[test]
fn sinr_decreases_with_more_interferers() {
    let signal_dbm = -70.0_f32;
    let noise_dbm = -100.0_f32;

    let sinr_no_interf = PropagationModel::sinr_db(signal_dbm, noise_dbm, &[]);
    let sinr_one_interf = PropagationModel::sinr_db(signal_dbm, noise_dbm, &[-80.0]);
    let sinr_two_interf = PropagationModel::sinr_db(signal_dbm, noise_dbm, &[-80.0, -75.0]);

    assert!(
        sinr_no_interf > sinr_one_interf,
        "SINR with 0 interferers ({}) should exceed SINR with 1 ({})",
        sinr_no_interf, sinr_one_interf
    );
    assert!(
        sinr_one_interf > sinr_two_interf,
        "SINR with 1 interferer ({}) should exceed SINR with 2 ({})",
        sinr_one_interf, sinr_two_interf
    );
}

/// shannon_capacity_mbps increases with higher SINR.
#[test]
fn shannon_capacity_increases_with_sinr() {
    let radio = default_radio();
    let cap_0  = PropagationModel::shannon_capacity_mbps(0.0, &radio);
    let cap_10 = PropagationModel::shannon_capacity_mbps(10.0, &radio);
    let cap_20 = PropagationModel::shannon_capacity_mbps(20.0, &radio);
    assert!(cap_0 < cap_10, "capacity at 0 dB ({}) should be < at 10 dB ({})", cap_0, cap_10);
    assert!(cap_10 < cap_20, "capacity at 10 dB ({}) should be < at 20 dB ({})", cap_10, cap_20);
}

/// SINR result must be finite and not NaN for typical input values.
#[test]
fn sinr_is_finite_and_not_nan() {
    let signal_dbm = -70.0_f32;
    let noise_dbm = -100.0_f32;
    let interferers = &[-85.0_f32, -90.0, -95.0];
    let sinr = PropagationModel::sinr_db(signal_dbm, noise_dbm, interferers);
    assert!(!sinr.is_nan(), "SINR must not be NaN");
    assert!(sinr.is_finite(), "SINR must be finite");
}
