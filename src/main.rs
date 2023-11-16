#[allow(unused_imports)]
use log::error;
#[allow(unused_imports)]
use log::info;
#[allow(unused_imports)]
use log::warn;

use esp_idf_sys as _;

use std::sync::Arc;
use std::sync::Mutex;

use esp_idf_hal::adc;
use esp_idf_hal::adc::AdcDriver;
use esp_idf_hal::adc::AdcChannelDriver;
use esp_idf_hal::adc::attenuation;

//use esp_idf_hal::delay::Ets;
use esp_idf_hal::delay::FreeRtos;

const MACHINE_NAME: &str = "peasant";

// Li-ion 3.7v -> 4.2v
const BATTERY_VOLTAGE: f32 = 3.7;
const VOLTAGE_COEFICIENT: f32 = 5.114;
const VOLTAGE_DIVIDER: &str = "4.1387v : 0.809 = 5.114 coeficient";

const REPETITION: u8 = 10;
const DELAY_MEASUREMENT_MS: u32 = 100; // 1000

//
fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    info!("machine: {MACHINE_NAME} -> rust_esp32_std_on_battery");
    info!("battery voltage: {BATTERY_VOLTAGE}");
    info!("voltage divider: {VOLTAGE_DIVIDER}");
    info!("voltage coeficient: {VOLTAGE_COEFICIENT}");
    
    // PERIPHERALS
    //
    let peripherals = esp_idf_hal::peripherals::Peripherals::take().unwrap();
    let adc_1 = peripherals.adc1;
    // todo!() -> channel/...
    let adc_peripherals_1 = Arc::new(Mutex::new(adc_1));
    //let adc_2 = peripherals.adc2;
    
    // PINS
    //
    //let pin_adc_0 = peripherals.pins.gpio0; // ADC1-0 GPIO0
    //let pin_adc_1 = peripherals.pins.gpio1; // ADC1-1 GPIO1
    //let pin_adc_2 = peripherals.pins.gpio2; // ADC1-2 GPIO2
    let pin_adc_3 = peripherals.pins.gpio3; // ADC1-3 GPI03 
    //let pin_adc_4 = peripherals.pins.gpio4; // ADC1-4 GPI04 
    //let pin_adc_5 = peripherals.pins.gpio5; // ADC2-0 GPIO5    

    // MEASUREMENT
    //
    let mut adc_channel_driver: AdcChannelDriver::<{ attenuation::DB_11 }, _> = AdcChannelDriver::new(pin_adc_3)?;

    let mut measurement_counter = 0;
    let mut measurement_values = Vec::new();
    
    match adc_peripherals_1
        .lock() {
            Ok(adc_peripheral) => {      
                let mut adc_driver = AdcDriver::new(
                    adc_peripheral,
                    &adc::config::Config::new().calibration(true),
                )?;
              
                while measurement_counter < REPETITION {
                    measurement_counter += 1;
                    
                    let measurement = adc_driver.read(&mut adc_channel_driver)?;
                    warn!("$$$ ADC?-?: [{measurement_counter:03}] {measurement} mV");
                    measurement_values.push(measurement);

                    FreeRtos::delay_ms(DELAY_MEASUREMENT_MS);
                }
                
                let average = measurement_values
                    .iter()
                    .sum::<u16>() as f32
                    / (measurement_values.len() as f32);
                
                let voltage = average * VOLTAGE_COEFICIENT;

                warn!("$$$ ADC -> average: {} mV / {}",
                      average,
                      voltage,
                );
            },
            Err(e) => {
                error!("error arc/mutex adc: {e:?}");
            },
        }

    // todo!() -> deep_sleep
    
    Ok(())
}

