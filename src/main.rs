mod battery;

#[allow(unused_imports)]
use log::error;
#[allow(unused_imports)]
use log::info;
#[allow(unused_imports)]
use log::warn;

use esp_idf_sys as _;

use esp_idf_hal::adc::AdcChannelDriver;
use esp_idf_hal::adc::attenuation;

const MACHINE_NAME: &str = "peasant";

const ATTN_ONE: u32 = attenuation::DB_11;
const ATTN_TWO: u32 = attenuation::DB_11;

//
fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    info!("machine: {MACHINE_NAME} -> rust_esp32_std_on_battery");
   
    // PERIPHERALS
    //
    let peripherals = esp_idf_hal::peripherals::Peripherals::take().unwrap();
    let adc_1 = peripherals.adc1;
    let adc_2 = peripherals.adc2;

    // todo!() -> channel/...
    
    // PINS
    //
    //let pin_adc_0 = peripherals.pins.gpio0; // ADC1-0 GPIO0
    //let pin_adc_1 = peripherals.pins.gpio1; // ADC1-1 GPIO1
    //let pin_adc_2 = peripherals.pins.gpio2; // ADC1-2 GPIO2
    let pin_adc_3 = peripherals.pins.gpio3; // ADC1-3 GPI03 
    //let pin_adc_4 = peripherals.pins.gpio4; // ADC1-4 GPI04 
    let pin_adc_5 = peripherals.pins.gpio5; // ADC2-0 GPIO5    

    // MEASUREMENT
    //
    // 1
    let mut adc_channel_driver_one: AdcChannelDriver::<ATTN_ONE, _> = AdcChannelDriver::new(pin_adc_3)?;

    if let Ok(()) = battery::measure(&mut adc_channel_driver_one,
                                     adc_1,
    ) {}

    // 2
    let mut adc_channel_driver_two: AdcChannelDriver::<ATTN_TWO, _> = AdcChannelDriver::new(pin_adc_5)?;

    if let Ok(()) = battery::measure(&mut adc_channel_driver_two,
                                     adc_2,
    ) {}
    
    // todo!() -> deep_sleep
    
    Ok(())
}

