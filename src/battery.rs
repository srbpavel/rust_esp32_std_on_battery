#[allow(unused_imports)]
use log::error;
#[allow(unused_imports)]
use log::info;
#[allow(unused_imports)]
use log::warn;

use esp_idf_hal::gpio::ADCPin;
use esp_idf_hal::peripheral::Peripheral;

use esp_idf_hal::adc;
use esp_idf_hal::adc::Adc;
use esp_idf_hal::adc::AdcChannelDriver;
use esp_idf_hal::adc::AdcDriver;

use esp_idf_hal::delay::FreeRtos;

//const BATTERY_VOLTAGE: f32 = 3.7;
const VOLTAGE_COEFICIENT: f32 = 5.114;
//const VOLTAGE_DIVIDER: &str = "4.1387v : 0.809 = 5.114 coeficient";

const REPETITION: u8 = 10;
const DELAY_MEASUREMENT_MS: u32 = 100; // 1000

//
pub fn measure<const ATTN: u32, PIN, ADC>(
    adc_channel_driver: &mut AdcChannelDriver<ATTN, PIN>,
    adc_peripheral: ADC,
) -> Result<(), esp_idf_sys::EspError> 
where
    ADC: Adc + Peripheral<P = ADC>, <ADC as Peripheral>::P: Adc,
    PIN: ADCPin<Adc = ADC>,
{
    let mut adc_driver = AdcDriver::new(
        adc_peripheral,
        &adc::config::Config::new().calibration(true),
    )?;

    let mut measurement_counter = 0;
    let mut measurement_values = Vec::new();
    
    while measurement_counter < REPETITION {
        measurement_counter += 1;
        
        let measurement = adc_driver.read(adc_channel_driver)?;

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
    
    Ok(())
}
