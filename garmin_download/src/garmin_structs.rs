

use serde::Deserialize;

#[allow(non_snake_case, dead_code)]
#[derive(Default, Deserialize)]
pub struct BioMetricProfile {
    pub userId: u64,
    pub height: f64,
    pub weight: f64,
    pub vo2Max: f64,
    pub vo2MaxCycling: Option<f64>,
    pub lactateThresholdHeartRate: Option<f64>,
    pub activityClass: Option<f64>,
    pub functionalThresholdPower: Option<f64>,
    pub criticalSwimSpeed: Option<f64>
}

#[allow(non_snake_case, dead_code)]
#[derive(Default, Deserialize)]
pub struct UserInfo {
    pub birthDate: String,
    pub genderType: String,
    pub email: String,
    pub locale: String,
    pub timeZone: String,
    pub age: u32,
    pub countryCode: String    
}

#[allow(non_snake_case, dead_code)]
#[derive(Default, Deserialize)]
pub struct PersonalInfo {
    pub userInfo: UserInfo,
    pub biometricProfile: BioMetricProfile,
    pub timeZone: String,
    pub locale: String,
    pub birthDate: String,
    pub gender: String,
}

