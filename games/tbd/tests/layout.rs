//! The flat arrays stay lined up with `assets/tbd/tbd.xml`.

use tbd::{CTRL_LEN, CTRL_NAMES, OBS_LEN, OBS_NAMES, SENSOR_LEN, SENSOR_SPEC};

fn section<'a>(xml: &'a str, open: &str, close: &str) -> &'a str {
    let start = xml.find(open).expect(open) + open.len();
    let end = xml[start..].find(close).expect(close) + start;
    &xml[start..end]
}

fn names(xml: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = xml.as_bytes();
    let key = b"name=\"";
    let mut i = 0;
    while i + key.len() < bytes.len() {
        let start_attr =
            bytes[i..].starts_with(key) && (i == 0 || !bytes[i - 1].is_ascii_alphanumeric());
        if start_attr {
            let value = &xml[i + key.len()..];
            let end = value.find('"').unwrap();
            out.push(value[..end].to_string());
            i += key.len() + end;
        } else {
            i += 1;
        }
    }
    out
}

#[test]
fn actuators_and_sensors_match_the_model_file() {
    let xml = include_str!("../../../assets/tbd/tbd.xml");
    let actuators = names(section(xml, "<actuator>", "</actuator>"));
    assert_eq!(actuators, CTRL_NAMES);
    assert_eq!(CTRL_NAMES.len(), CTRL_LEN);

    let sensors = names(section(xml, "<sensor>", "</sensor>"));
    let spec_names: Vec<&str> = SENSOR_SPEC.iter().map(|(n, _)| *n).collect();
    assert_eq!(sensors, spec_names);
    let width: usize = SENSOR_SPEC.iter().map(|(_, w)| *w).sum();
    assert_eq!(width, SENSOR_LEN);
    assert_eq!(OBS_LEN, SENSOR_LEN + 3 + 3 + 4);
    assert_eq!(tbd::OBS_LEN_V1, 20);
    assert_eq!(OBS_NAMES.len(), OBS_LEN);
    assert!(xml.contains("timestep=\"0.002\""));
    assert!(xml.contains("meshdir=\"meshes\""));
}
