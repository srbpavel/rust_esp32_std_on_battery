mod battery;

#[allow(unused_imports)]
use log::error;
#[allow(unused_imports)]
use log::info;
#[allow(unused_imports)]
use log::warn;

use std::sync::mpsc::channel;

use embedded_hal::blocking::delay::DelayMs;

use esp_idf_sys as _;

use esp_idf_hal::adc::attenuation;
//use esp_idf_hal::adc::AdcChannelDriver;


// todo!() -> Config
const MACHINE_NAME: &str = "peasant";

const DELAY_SLEEP_DURATION_MS: u32 = 10*1000;

const ATTN_ONE: u32 = attenuation::DB_11;
const ATTN_TWO: u32 = attenuation::DB_11;


//
fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    info!("machine: {MACHINE_NAME} -> rust_esp32_std_on_battery");

    let mut sleep = esp_idf_hal::delay::FreeRtos {};
    
    // PERIPHERALS
    //
    let peripherals = esp_idf_hal::peripherals::Peripherals::take().unwrap();
    let adc_1 = peripherals.adc1;
    let adc_1 = std::sync::Arc::new(std::sync::Mutex::new(adc_1));
    let adc_2 = peripherals.adc2;
    let adc_2 = std::sync::Arc::new(std::sync::Mutex::new(adc_2));

    //let mut adc_1 = peripherals.adc1;

    // CHANNEL
    let (command_sender, command_receiver) =
        channel::<battery::Command>();

    let command_receiver = std::sync::Arc::new(std::sync::Mutex::new(command_receiver));
    
    let (measurement_sender, measurement_receiver) =
        channel::<battery::Measurement>();
    /*
    let (command_sender_one, command_receiver_one) =
        channel::<battery::Command>();

    let (command_sender_two, command_receiver_two) =
        channel::<battery::Command>();

    let (measurement_sender_one, measurement_receiver_one) =
        channel::<battery::Measurement>();
    */
    
    // PINS
    //
    // ADC1
    //let pin_adc_0 = peripherals.pins.gpio0; // ADC1-0 GPIO0
    //let pin_adc_1 = peripherals.pins.gpio1; // ADC1-1 GPIO1
    //let pin_adc_2 = peripherals.pins.gpio2; // ADC1-2 GPIO2
    let pin_adc_3 = peripherals.pins.gpio3; // ADC1-3 GPI03 
    let pin_adc_4 = peripherals.pins.gpio4; // ADC1-4 GPI04 
    // ADC2
    let pin_adc_5 = peripherals.pins.gpio5; // ADC2-0 GPIO5    

    // MEASUREMENT
    //
    // 1
    //
    // INFINITE
    //
    // todo!() -> via conf?
    /*
    let delay_measurement_ms: u32 = 100;
    battery::start_via_pin::<_, _, ATTN_ONE>(
        pin_adc_4,
        adc_1,
        delay_measurement_ms,
        command_receiver_one,
        measurement_sender_one.clone(),
    );
    */

    let battery_one_3: battery::Battery<_, _, ATTN_ONE> = battery::Battery {
        gpio: pin_adc_3,
        adc: adc_1.clone(),
        delay_ms: 100,
        //receiver: command_receiver_one,
        receiver: command_receiver.clone(),
        //sender: measurement_sender_one.clone(),
        sender: measurement_sender.clone(),
    };

    battery_one_3.init();

    let battery_one_4: battery::Battery<_, _, ATTN_ONE> = battery::Battery {
        gpio: pin_adc_4,
        adc: adc_1.clone(),
        delay_ms: 100,
        //receiver: command_receiver_one,
        receiver: command_receiver.clone(),
        //sender: measurement_sender_one.clone(),
        sender: measurement_sender.clone(),
    };

    battery_one_4.init();
    
    let battery_two_5: battery::Battery<_, _, ATTN_TWO> = battery::Battery {
        gpio: pin_adc_5,
        //adc: adc_2,
        //adc: std::sync::Arc::new(std::sync::Mutex::new(adc_2)),
        adc: adc_2.clone(),
        delay_ms: 200,
        //receiver: command_receiver_two,
        receiver: command_receiver.clone(),
        //sender: measurement_sender_one.clone(),
        sender: measurement_sender.clone(),
    };

    battery_two_5.init();

    /*
    let battery_three: battery::Battery<_, _, ATTN_ONE> = battery::Battery {
        gpio: pin_adc_3,
        adc: adc_1,
        delay_ms: 300,
        receiver: command_receiver_one,
        sender: measurement_sender_one.clone(),
    };

    battery_two.init();
    */
    
    // ONCE
    /*
    // via AdcChannelDriver    
    let mut adc_channel_driver_one: AdcChannelDriver::<ATTN_ONE, _> = AdcChannelDriver::new(pin_adc_3)?;

    if let Err(_e) = battery::measure_channel_driver(&mut adc_channel_driver_one,
                                                     adc_1,
                                                     //&mut adc_1,
    ) {}
    */
    
    /*
    // 2
    // via Gpio
    if let Err(_e) = battery::measure_pin::<_, _, ATTN_TWO>(pin_adc_5,
                                                            adc_2,
    ) {}
    */
    
    // MEASUREMENT display/parse/mqtt/...
    std::thread::spawn(move || {
        //while let Ok(channel_data) = measurement_receiver_one.recv() {
        while let Ok(channel_data) = measurement_receiver.recv() {
            info!("MEASUREMENT value: {:?}",
                  channel_data,
            );
        }
    });
    //

    // todo!() -> prepare for mqtt request
    // try harder !!!
    // LOOP
    std::thread::spawn(move || {
        loop {
            if let Err(e) = command_sender
                .clone()
                .send(battery::Command::Measure) {
                    
                    error!("### Error: One send(Command::Measure) -> {e:?}");
                }

            /*
            if let Err(e) = command_sender_two
                .clone()
                .send(battery::Command::Measure) {
                    
                    error!("### Error: Two send(Command::Measure) -> {e:?}");
                }
            */
            
            sleep.delay_ms(DELAY_SLEEP_DURATION_MS);
        }
    });

    // todo!() -> deep_sleep
    // via config measure once + sleep
    // via config infinite measuare
    
    Ok(())
}

