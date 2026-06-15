//! Virtual parameter service for the `rt_rc` dialect.

mod common;

use common::*;

struct SimulatedParam {
    id: &'static str,
    value: f32,
    index: u16,
}

fn main() {
    let dialect = MavlinkDialectRtRc;

    let list_request = ParamRequestList {
        target_system: DRONE_SYSTEM_ID,
        target_component: DRONE_COMPONENT_ID,
    };
    let list_frame = frame_from_gcs(Box::new(list_request.clone()), 1);
    log_frame("GCS ->", &list_frame);
    let _ = round_trip_message(&dialect, &list_request);

    let simulated_params = [
        SimulatedParam {
            id: "SYSID_THISMAV",
            value: 1.0,
            index: 0,
        },
        SimulatedParam {
            id: "SYSID_MYGCS",
            value: 255.0,
            index: 1,
        },
        SimulatedParam {
            id: "COMPASS_ENABLE",
            value: 1.0,
            index: 2,
        },
    ];

    for param in &simulated_params {
        let value = ParamValue {
            param_value: param.value,
            param_count: simulated_params.len() as u16,
            param_index: param.index,
            param_id: param_id_from_string(param.id),
            param_type: MavParamType::MAV_PARAM_TYPE_REAL32,
        };
        let value_frame = frame_from_drone(Box::new(value.clone()), param.index as u8 + 10);
        log_frame("Drone ->", &value_frame);
        if let Some(parsed) = round_trip_message(&dialect, &value) {
            if let Some(parsed_value) = downcast_message::<ParamValue>(parsed.as_ref()) {
                println!(
                    "  PARAM_VALUE [{}/{}] {}={}",
                    param.index + 1,
                    simulated_params.len(),
                    param_id_to_string(&parsed_value.param_id),
                    parsed_value.param_value
                );
            }
        }
    }

    let param_name = "SYSID_THISMAV";
    let read_request = ParamRequestRead {
        param_index: -1,
        target_system: DRONE_SYSTEM_ID,
        target_component: DRONE_COMPONENT_ID,
        param_id: param_id_from_string(param_name),
    };
    let read_frame = frame_from_gcs(Box::new(read_request.clone()), 50);
    log_frame("GCS ->", &read_frame);
    if let Some(parsed) = round_trip_message(&dialect, &read_request) {
        if let Some(parsed_read) = downcast_message::<ParamRequestRead>(parsed.as_ref()) {
            println!(
                "  PARAM_REQUEST_READ id={}",
                param_id_to_string(&parsed_read.param_id)
            );
        }
    }

    let single_value = ParamValue {
        param_value: 1.0,
        param_count: simulated_params.len() as u16,
        param_index: 0,
        param_id: param_id_from_string(param_name),
        param_type: MavParamType::MAV_PARAM_TYPE_REAL32,
    };
    let single_frame = frame_from_drone(Box::new(single_value.clone()), 51);
    log_frame("Drone ->", &single_frame);
    let _ = round_trip_message(&dialect, &single_value);
}
