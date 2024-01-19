use std::collections::{HashMap, HashSet};

// this map provides all (currently known) FitDataFields for each FitDataRecordType
#[allow(dead_code)]
pub fn get_activity_map() -> HashMap<&'static str, HashSet<&'static str>>  { 
    HashMap::from([
        ("user_profile", HashSet::from(["unknown_field_43", "hr_setting", "speed_setting", "depth_setting", "language", "position_setting", "unknown_field_45", "unknown_field_37", "unknown_field_60", "temperature_setting", "unknown_field_54", "sleep_time", "unknown_field_58", "elev_setting", "weight_setting", "unknown_field_62", "height", "resting_heart_rate", "unknown_field_24", "wake_time", "dist_setting", "unknown_field_44", "activity_class", "gender", "unknown_field_52", "weight", "unknown_field_53", "unknown_field_57", "height_setting", "unknown_field_33"])),
        ("file_id", HashSet::from(["time_created", "type", "garmin_product", "manufacturer", "serial_number"])),
        ("event", HashSet::from(["event", "timestamp", "timer_trigger", "event_type", "event_group"])),
        ("zones_target", HashSet::from(["max_heart_rate", "threshold_heart_rate", "pwr_calc_type", "unknown_field_10", "hr_calc_type", "unknown_field_13", "unknown_field_11", "unknown_field_12", "unknown_field_9", "unknown_field_254", "functional_threshold_power"])),
        ("sport", HashSet::from(["unknown_field_13", "sport", "sub_sport", "name", "unknown_field_5", "unknown_field_6", "unknown_field_11"])),
        ("device_settings", HashSet::from(["unknown_field_141", "unknown_field_124", "unknown_field_41", "unknown_field_43", "unknown_field_109", "unknown_field_112", "unknown_field_164", "unknown_field_180", "unknown_field_211", "unknown_field_212", "unknown_field_126", "unknown_field_68", "lactate_threshold_autodetect_enabled", "time_zone_offset", "unknown_field_205", "unknown_field_206", "mounting_side", "unknown_field_53", "unknown_field_219", "unknown_field_200", "date_mode", "unknown_field_128", "unknown_field_199", "unknown_field_127", "unknown_field_177", "unknown_field_108", "unknown_field_162", "unknown_field_201", "unknown_field_15", "unknown_field_243", "utc_offset", "time_offset", "unknown_field_101", "unknown_field_22", "unknown_field_52", "unknown_field_135", "unknown_field_218", "unknown_field_69", "activity_tracker_enabled", "unknown_field_63", "unknown_field_81", "unknown_field_161", "unknown_field_149", "unknown_field_181", "unknown_field_67", "unknown_field_87", "unknown_field_208", "unknown_field_178", "unknown_field_209", "unknown_field_65", "unknown_field_83", "unknown_field_11", "unknown_field_217", "time_mode", "unknown_field_139", "unknown_field_26", "unknown_field_45", "unknown_field_66", "unknown_field_82", "unknown_field_84", "unknown_field_160", "unknown_field_143", "unknown_field_145", "autosync_min_time", "unknown_field_204", "backlight_mode", "unknown_field_13", "unknown_field_125", "unknown_field_48", "autosync_min_steps", "active_time_zone", "unknown_field_144", "unknown_field_207", "unknown_field_110", "unknown_field_203", "unknown_field_179", "unknown_field_111", "unknown_field_163", "unknown_field_64", "unknown_field_138", "unknown_field_210", "unknown_field_14", "unknown_field_10", "unknown_field_107", "unknown_field_44", "unknown_field_85", "auto_activity_detect", "unknown_field_35", "unknown_field_75", "move_alert_enabled", "unknown_field_3", "unknown_field_42", "unknown_field_133"])),
        ("time_in_zone", HashSet::from(["hr_calc_type", "reference_index", "time_in_hr_zone", "threshold_heart_rate", "timestamp", "reference_mesg", "hr_zone_high_boundary", "max_heart_rate", "resting_heart_rate"])),
        ("record", HashSet::from(["timestamp", "unknown_field_136", "unknown_field_141", "heart_rate", "resistance", "distance", "unknown_field_135", "enhanced_respiration_rate"])),
        ("session", HashSet::from(["unknown_field_188", "total_cycles", "start_time", "enhanced_min_respiration_rate", "enhanced_max_respiration_rate", "total_training_effect", "unknown_field_138", "message_index", "enhanced_avg_respiration_rate", "total_elapsed_time", "num_laps", "unknown_field_81", "first_lap_index", "total_calories", "max_heart_rate", "unknown_field_151", "total_anaerobic_training_effect", "training_load_peak", "avg_heart_rate", "sport_profile_name", "sub_sport", "unknown_field_184", "timestamp", "total_timer_time", "enhanced_avg_speed", "sport", "trigger", "total_distance", "unknown_field_196", "event", "unknown_field_152", "event_type"])),
        ("file_creator", HashSet::from(["software_version", "unknown_field_2"])),
        ("lap", HashSet::from(["start_time", "max_heart_rate", "total_timer_time", "enhanced_avg_speed", "enhanced_avg_respiration_rate", "sub_sport", "message_index", "sport", "avg_heart_rate", "total_calories", "enhanced_max_respiration_rate", "event", "total_distance", "timestamp", "total_cycles", "lap_trigger", "event_type", "total_elapsed_time", "unknown_field_155"])),
        ("activity", HashSet::from(["type", "total_timer_time", "event_type", "event", "local_timestamp", "timestamp", "num_sessions", "unknown_field_8"])),
        ("device_info", HashSet::from(["device_index", "local_device_type", "unknown_field_17", "battery_level", "serial_number", "unknown_field_24", "software_version", "ant_network", "battery_status", "unknown_field_30", "garmin_product", "timestamp", "antplus_device_type", "unknown_field_15", "product", "source_type", "ble_device_type", "unknown_field_29", "manufacturer"])),
   ])
}

#[allow(dead_code)]
pub fn get_monitoring_map() -> HashMap<&'static str, HashSet<&'static str>>  { 
    HashMap::from([
        ("monitoring", HashSet::from(["unknown_field_36", "intensity", "timestamp_16", "activity_type", "steps", "active_calories", "active_time", "unknown_field_38", "timestamp", "unknown_field_35", "distance", "unknown_field_37", "duration_min", "heart_rate"])),
        ("ohr_settings", HashSet::from(["enabled", "timestamp"])),
        ("stress_level", HashSet::from(["stress_level_time", "unknown_field_3", "unknown_field_4", "unknown_field_2", "stress_level_value"])),
        ("monitoring_info", HashSet::from(["cycles_to_calories", "resting_metabolic_rate", "unknown_field_7", "timestamp", "activity_type", "local_timestamp", "cycles_to_distance"])),
        ("file_id", HashSet::from(["garmin_product", "number", "serial_number", "time_created", "type", "manufacturer"])),
        ("event", HashSet::from(["event", "auto_activity_detect_start_timestamp", "activity_type", "auto_activity_detect_duration", "event_type", "data", "timestamp"])),
        ("respiration_rate", HashSet::from(["timestamp", "respiration_rate"])),
        ("monitoring_hr_data", HashSet::from(["resting_heart_rate", "timestamp", "current_day_resting_heart_rate"])),
        ("device_info", HashSet::from(["serial_number", "timestamp", "software_version", "garmin_product", "manufacturer"])),
        ("sleep_level", HashSet::from(["sleep_level", "timestamp"])),
        ("sleep_assessment", HashSet::from(["combined_awake_score", "deep_sleep_score", "overall_sleep_score", "unknown_field_13", "sleep_duration_score", "sleep_recovery_score", "awakenings_count", "sleep_restlessness_score", "interruptions_score", "unknown_field_16", "unknown_field_12", "average_stress_during_sleep", "sleep_quality_score", "rem_sleep_score", "awakenings_count_score", "light_sleep_score", "awake_time_score"])),
        ("hrv_status_summary", HashSet::from(["unknown_field_7", "baseline_balanced_upper", "last_night_5_min_high", "timestamp", "last_night_average", "status", "baseline_balanced_lower", "weekly_average", "baseline_low_upper", "unknown_field_8"])),
        ("hrv_value", HashSet::from(["value", "timestamp"])) 
    ])
}