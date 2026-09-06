use std::fs;
use std::path::Path;

use nebudcs_scripting::library::import_export::{
    export_package, export_template, SerializationFormat,
};
use nebudcs_scripting::library::standard::{
    load_all_standard_packages, load_all_standard_templates,
};

#[test]
fn export_and_verify_standard_library_fixtures() {
    let base_dir = Path::new("../../packages/standard-library");
    // If running from cargo test in crates/nebudcs-scripting, base_dir is ../../packages/standard-library
    let target_dir = if base_dir.exists() {
        base_dir.to_path_buf()
    } else {
        Path::new("packages/standard-library").to_path_buf()
    };

    fs::create_dir_all(&target_dir).unwrap();

    let templates = load_all_standard_templates().expect("Standard templates must build");
    assert_eq!(templates.len(), 12);

    for tmpl in &templates {
        let json_content =
            export_template(tmpl, SerializationFormat::Json).expect("Export template to JSON");
        let toml_content =
            export_template(tmpl, SerializationFormat::Toml).expect("Export template to TOML");

        // Category folder
        let subfolder = match tmpl.id.as_str() {
            "standard.pump.centrifugal" => "pumps",
            "standard.compressor.centrifugal" => "compressors",
            "standard.fan.centrifugal" => "fans",
            "standard.valve.control_globe" => "valves",
            "standard.vessel.vertical_separator" => "vessels",
            "standard.tank.atmospheric" => "tanks",
            "standard.heat_exchanger.shell_and_tube" => "heat_exchangers",
            "standard.heater.fired_process" => "heaters",
            "standard.reactor.cstr" => "reactors",
            "standard.mixer.static_inline" => "mixers",
            "standard.splitter.flow_splitter" => "splitters",
            "standard.sensor.transmitter" => "sensors",
            _ => "misc",
        };

        let cat_dir = target_dir.join(subfolder);
        fs::create_dir_all(&cat_dir).unwrap();

        let filename_base = tmpl.id.replace("standard.", "").replace('.', "_");
        fs::write(cat_dir.join(format!("{filename_base}.json")), json_content).unwrap();
        fs::write(cat_dir.join(format!("{filename_base}.toml")), toml_content).unwrap();
    }

    let packages = load_all_standard_packages();
    assert_eq!(packages.len(), 2);

    let pkg_dir = target_dir.join("packages");
    fs::create_dir_all(&pkg_dir).unwrap();

    for pkg in &packages {
        let json_content =
            export_package(pkg, SerializationFormat::Json).expect("Export package to JSON");
        let toml_content =
            export_package(pkg, SerializationFormat::Toml).expect("Export package to TOML");

        let filename_base = pkg.id.replace("standard.package.", "");
        fs::write(pkg_dir.join(format!("{filename_base}.json")), json_content).unwrap();
        fs::write(pkg_dir.join(format!("{filename_base}.toml")), toml_content).unwrap();
    }
}
