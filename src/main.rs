// https://docs.espressif.com/projects/esp-idf/en/v4.4.6/esp32c3/api-reference/peripherals/adc.html

#[allow(unused_imports)]
use log::error;
#[allow(unused_imports)]
use log::info;
#[allow(unused_imports)]
use log::warn;

mod battery;

use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::sync::Mutex;

use embedded_hal::blocking::delay::DelayMs;

use esp_idf_sys as _;

use esp_idf_hal::gpio::Pin;

use esp_idf_hal::delay::FreeRtos;
//use esp_idf_hal::delay::Ets;

use esp_idf_hal::adc::ADC1;
//use esp_idf_hal::adc::ADC2;
use esp_idf_hal::adc::attenuation;
use esp_idf_hal::adc::AdcChannelDriver;

// todo!() -> Config
const MACHINE_NAME: &str = "peasant";

pub const ADC_READ_REPETITION: u8 = 10;

const DELAY_SLEEP_DURATION_MS: u32 = 30*1000;
const DELAY_COMMAND_DURATION_MS: u32 = 100;
pub const DELAY_MEASUREMENT_MS: u32 = 100;

//const BATTERY_VOLTAGE: f32 = 4.2;
//const VOLTAGE_DIVIDER: &str = "4.134v : 0.810 = 5.10 coeficient";
//const VOLTAGE_DIVIDER_COEFICIENT_DEFAULT: f32 = 5.0;
const VOLTAGE_DIVIDER_COEFICIENT_GPIO0: f32 = 4.97;
const VOLTAGE_DIVIDER_COEFICIENT_GPIO1: f32 = 5.03;
const VOLTAGE_DIVIDER_COEFICIENT_GPIO2: f32 = 5.0;  
const VOLTAGE_DIVIDER_COEFICIENT_GPIO3: f32 = 5.1;
const VOLTAGE_DIVIDER_COEFICIENT_GPIO4: f32 = 5.11;
//const VOLTAGE_DIVIDER_COEFICIENT_GPIO5: f32 = 5.0;

//const BATTERY_WARNING_BOUNDARY_DEFAULT: f32 = 3700.0;
const BATTERY_WARNING_BOUNDARY_GPIO0: f32 = 3500.0;
const BATTERY_WARNING_BOUNDARY_GPIO1: f32 = 3500.0;
const BATTERY_WARNING_BOUNDARY_GPIO2: f32 = 3500.0;
const BATTERY_WARNING_BOUNDARY_GPIO3: f32 = 3500.0;
const BATTERY_WARNING_BOUNDARY_GPIO4: f32 = 3500.0;
//const BATTERY_WARNING_BOUNDARY_GPIO5: f32 = 3500.0;

/*
ADC_ATTEN_DB_0   0 mV ~ 750 mV
ADC_ATTEN_DB_2_5 0 mV ~ 1050 mV
ADC_ATTEN_DB_6   0 mV ~ 1300 mV
ADC_ATTEN_DB_11  0 mV ~ 2500 mV
*/
const ATTN_ONE: u32 = attenuation::DB_2_5;
//const ATTN_TWO: u32 = attenuation::DB_2_5;

//
fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    info!("machine: {MACHINE_NAME} -> rust_esp32_std_on_battery");

    let delay_between_samples = FreeRtos {};
    //let delay_between_samples = Ets {};
    let mut delay_after_measure = FreeRtos{};
    
    // PERIPHERALS
    //
    let peripherals = esp_idf_hal::peripherals::Peripherals::take().unwrap();
    let adc_1 = peripherals.adc1;
    let adc_1 = Arc::new(Mutex::new(adc_1));
    /*
    E (3482) ADC: ADC2 is no longer supported, please use ADC1. Search for errata on espressif website for more details. You can enable ADC_ONESHOT_FORCE_USE_ADC2_ON_C3 to force use ADC2
E (3492) ADC: adc2_get_raw(750): adc unit not supporte
    */
    //let adc_2 = peripherals.adc2;

    // CHANNEL
    let (command_sender, command_receiver) = channel::<battery::Command>();
    let (measurement_sender, measurement_receiver) = channel::<battery::Measurement>();
    
    // PINS
    //
    // ADC1
    let pin_adc_0 = peripherals.pins.gpio0; // ADC1-0 GPIO0
    let pin_adc_1 = peripherals.pins.gpio1; // ADC1-1 GPIO1
    let pin_adc_2 = peripherals.pins.gpio2; // ADC1-2 GPIO2
    let pin_adc_2 = Arc::new(Mutex::new(pin_adc_2));
    let pin_adc_3 = peripherals.pins.gpio3; // ADC1-3 GPI03 
    let pin_adc_4 = peripherals.pins.gpio4; // ADC1-4 GPI04
    // ADC2
    //let pin_adc_5 = peripherals.pins.gpio5; // ADC2-0 GPIO5    

    // MEASUREMENT -> display/parse/mqtt publish/...
    start_measurement_listener(measurement_receiver);
    
    //MEASURE via PIN ONCE
    warn!("MEASURE via PIN: start");
    if let Err(_e) = battery::measure_pin_once::<_, ADC1, ATTN_ONE, _> (
        pin_adc_2.clone(),
        adc_1.clone(),
        measurement_sender.clone(),
        VOLTAGE_DIVIDER_COEFICIENT_GPIO2,
        BATTERY_WARNING_BOUNDARY_GPIO2,
        &mut delay_after_measure,
    ) {}
    warn!("MEASURE via PIN: end + sleep/wait");
    FreeRtos{}.delay_ms(5*1000_u32);
    //_
  
    //MEASURE via ADC_CHANNEL_DRIVER ONCE
    let pin_id = pin_adc_3.pin();
    let mut adc_channel_driver_three: AdcChannelDriver::<ATTN_ONE, _> = AdcChannelDriver::new(pin_adc_3)?;
    //let mut adc_channel_driver_five: AdcChannelDriver::<ATTN_TWO, _> = AdcChannelDriver::new(pin_adc_5)?;

    let adc_1_clone = adc_1.clone();
    std::thread::spawn(move || {
        warn!("MEASURE via ADC_CHANNEL_DRIVER: start");

        if let Err(_e) = battery::measure_channel_driver_once::<ATTN_ONE, _, ADC1, _> (
        //if let Err(_e) = battery::measure_channel_driver_once::<ATTN_TWO, _, ADC2, _> (
            pin_id,
            &mut adc_channel_driver_three,
            //&mut adc_channel_driver_five,
            adc_1_clone,//.clone(),
            //adc_2,
            //measurement_sender.clone(),
            VOLTAGE_DIVIDER_COEFICIENT_GPIO3,
            BATTERY_WARNING_BOUNDARY_GPIO3,
            &mut FreeRtos{},
    ) {}
        warn!("MEASURE via ADC_CHANNEL_DRIVER: end + sleep/wait");
    });
    FreeRtos{}.delay_ms(5*1000_u32);
    //_
    
    // COMMAND producer -> just to have some samples
    start_command_producer(command_sender,
                           delay_between_samples,
    );
    
    // COMMAND listener
    info!("LISTEN for COMMAND");
    
    let mut sensor_gpio0 = battery::Sensor::<_, ADC1, ATTN_ONE>::new(
    //let mut sensor_gpio0 = battery::Sensor::<_, ADC1, ATTN_ONE, FreeRtos>::new(
        pin_adc_0,
        adc_1.clone(),
        measurement_sender.clone(),
        VOLTAGE_DIVIDER_COEFICIENT_GPIO0,
        //&mut delay,
        BATTERY_WARNING_BOUNDARY_GPIO0,
    )?;
    
    let mut sensor_gpio1 = battery::Sensor::<_, ADC1, ATTN_ONE>::new(
    //let mut sensor_gpio1 = battery::Sensor::<_, ADC1, ATTN_ONE, FreeRtos>::new(
        pin_adc_1,
        adc_1.clone(),
        measurement_sender.clone(),
        VOLTAGE_DIVIDER_COEFICIENT_GPIO1,
        //&mut delay,
        BATTERY_WARNING_BOUNDARY_GPIO1,
    )?;

    /* // gpio used for MEASUER via PIN ONCE
    let mut sensor_gpio2 = battery::Sensor::<_, ADC1, ATTN_ONE>::new(
    //let mut sensor_gpio2 = battery::Sensor::<_, ADC1, ATTN_ONE, FreeRtos>::new(
        pin_adc_2,
        adc_1.clone(),
        measurement_sender.clone(),
        VOLTAGE_DIVIDER_COEFICIENT_DEFAULT,
        //&mut delay,
        BATTERY_WARNING_BOUNDARY_DEFAULT,
    )?;
    */

    /* // gpio used for MEASURE via ADC_CHANNEL_DRIVER ONCE
    let mut sensor_gpio3 = battery::Sensor::<_, ADC1, ATTN_ONE>::new(
    //let mut sensor_gpio3 = battery::Sensor::<_, ADC1, ATTN_ONE, FreeRtos>::new(
        pin_adc_3,
        adc_1.clone(),
        measurement_sender.clone(),
        VOLTAGE_DIVIDER_COEFICIENT_GPIO,
        //&mut delay,
        BATTERY_WARNING_BOUNDARY_GPIO,
    )?;
    */
    
    let mut sensor_gpio4 = battery::Sensor::<_, ADC1, ATTN_ONE>::new(
    //let mut sensor_gpio4 = battery::Sensor::<_, ADC1, ATTN_ONE, FreeRtos>::new(
        pin_adc_4,
        adc_1.clone(),
        measurement_sender.clone(),
        //5.11,
        VOLTAGE_DIVIDER_COEFICIENT_GPIO4,
        //&mut delay,
        BATTERY_WARNING_BOUNDARY_GPIO4,
    )?;

    // COMMAND listen and MEASURE
    std::thread::spawn(move || {
        info!("thread LISTEN + MEASURE");

        while let Ok(channel_data) = command_receiver.recv() {
            info!("COMMAND value: {:?}",
                  channel_data,
            );
            
            match channel_data {
                battery::Command::Measure(pin_id) => {
                    match pin_id {
                        0 => if let Err(_e) = sensor_gpio0.measure(&mut delay_after_measure) {},
                        1 => if let Err(_e) = sensor_gpio1.measure(&mut delay_after_measure) {},
                        //2 => if let Err(_e) = sensor_gpio2.measure(&mut delay_after_measure) {},
                        //3 => if let Err(_e) = sensor_gpio3.measure(&mut delay_after_measure) {},
                        4 => if let Err(_e) = sensor_gpio4.measure(&mut delay_after_measure) {},
                        //5 => if let Err(_e) = sensor_gpio5.measure() {}
                        _ => {},
                    }
                },
            }
        }
    });

    // todo!() -> deep_sleep
    // via config measure once + sleep
    // via config infinite measuare
    
    Ok(())
}

//
fn start_measurement_listener(
    measurement_receiver: Receiver<battery::Measurement>,
) {
    std::thread::spawn(move || {
        info!("thread MEASUREMENT");
        
        while let Ok(channel_data) = measurement_receiver.recv() {
            info!("MEASUREMENT value: {:?}",
                  channel_data,
            );

            /*
            if channel_data.get_voltage() < BATTERY_WARNING_BOUNDARY {
                error!("BATTERY too low, replace with new !!!");
            }
            */
        }
    });
}

//
fn start_command_producer<D>(command_sender: Sender<battery::Command>,
                             mut delay: D
)
where
    D: embedded_hal::blocking::delay::DelayMs<u32> + std::marker::Send + 'static,
{
    std::thread::spawn(move || {
        info!("thread LOOP -> Command::Measure(pin_id)");
        
        loop {
            // ADC_1
            if let Err(e) = command_sender
                .clone()
                .send(battery::Command::Measure(3i32)) {
                    error!("### Error: Send(Command::Measure) -> {e:?}");
                }
            delay.delay_ms(DELAY_COMMAND_DURATION_MS);
            
            if let Err(e) = command_sender
                .clone()
                .send(battery::Command::Measure(4i32)) {
                    error!("### Error: Send(Command::Measure) -> {e:?}");
                }
            delay.delay_ms(DELAY_COMMAND_DURATION_MS);
            
            if let Err(e) = command_sender
                .clone()
                .send(battery::Command::Measure(0i32)) {
                    error!("### Error: Send(Command::Measure) -> {e:?}");
                }
            delay.delay_ms(DELAY_COMMAND_DURATION_MS);
            
            if let Err(e) = command_sender
                .clone()
                .send(battery::Command::Measure(1i32)) {
                    error!("### Error: Send(Command::Measure) -> {e:?}");
                }

            delay.delay_ms(DELAY_COMMAND_DURATION_MS);
            
            if let Err(e) = command_sender
                .clone()
                .send(battery::Command::Measure(2i32)) {
                    error!("### Error: Send(Command::Measure) -> {e:?}");
                }
            delay.delay_ms(DELAY_COMMAND_DURATION_MS);
            
            /*
            // ADC_2
            if let Err(e) = command_sender
                .clone()
                .send(battery::Command::Measure(5i32)) {
                    error!("### Error: Send(Command::Measure) -> {e:?}");
                }
            sleep.delay_ms(100_u32);
            */
            
            delay.delay_ms(DELAY_SLEEP_DURATION_MS);
        }
    });
}
