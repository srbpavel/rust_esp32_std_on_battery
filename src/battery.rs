#[allow(unused_imports)]
use log::error;
#[allow(unused_imports)]
use log::info;
#[allow(unused_imports)]
use log::warn;

use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;

use esp_idf_hal::gpio::IOPin;
use esp_idf_hal::gpio::ADCPin;
use esp_idf_hal::peripheral::Peripheral;

use esp_idf_hal::adc;
use esp_idf_hal::adc::Adc;
use esp_idf_hal::adc::AdcChannelDriver;
use esp_idf_hal::adc::AdcDriver;

use esp_idf_hal::delay::FreeRtos;

//const BATTERY_VOLTAGE: f32 = 3.7;
//const BATTERY_VOLTAGE: f32 = 4.2;
//const VOLTAGE_DIVIDER: &str = "4.1387v : 0.809 = 5.114 coeficient";
const VOLTAGE_COEFICIENT: f32 = 5.114;

const REPETITION: u8 = 10;
// todo!() conf of + fn
const DELAY_MEASUREMENT_MS: u32 = 100; // 1000

/*
pub enum Status {
    Init,
    Done,
    Failed,
}
*/

#[allow(unused)]
//
// https://users.rust-lang.org/t/how-to-store-a-trait-as-field-of-a-struct/87762/2
//
pub struct Battery<PIN, ADC, const ATTN: u32> {
    pub gpio: PIN,
    // todo!() -> try harder:
    pub adc: std::sync::Arc<std::sync::Mutex<ADC>>,
    pub delay_ms: u32,
    pub receiver: std::sync::Arc<std::sync::Mutex<Receiver<Command>>>,
    pub sender: Sender<Measurement>,
}

impl<PIN, ADC, const ATTN: u32> Battery<PIN, ADC, ATTN>
where
    ADC: Adc + Peripheral<P = ADC> + 'static, <ADC as Peripheral>::P: Adc,
    PIN: IOPin + ADCPin<Adc = ADC>,
{
    //
    pub fn init(self) {
        std::thread::spawn(move || {
            let pin_id = self.gpio.pin();
            let adc_channel_driver: Result<AdcChannelDriver::< ATTN, _>, esp_idf_sys::EspError> = AdcChannelDriver::new(self.gpio);
            
            match adc_channel_driver {
                Ok(mut cd) => {
                    let adc_driver = AdcDriver::new(
                        //self.adc,
                        self.adc
                            .lock()
                            .unwrap(), // NOT SAFE !!! todo
                        &adc::config::Config::new().calibration(true),
                    );
                    
                    if let Ok(mut d) = adc_driver {
                        while let Ok(channel_data) =
                            self.receiver
                            .lock()
                            .unwrap()
                            .recv() {
                                match channel_data {
                                    Command::Measure => {
                                        let mut measurement = Measurement::default();
                                        // todo!() via conf?
                                        let mut counter = 0;
                                        let mut values = Vec::new();
                                        
                                        while counter < REPETITION {
                                            counter += 1;
                                            
                                            match d.read(&mut cd) {
                                                Ok(value) => {
                                                    // DEBUG
                                                    warn!("$$$ PIN: {pin_id} [{counter:03}] {value} mV");
                                                    values.push(value);
                                                    
                                                },
                                                Err(_e) => {
                                                    // todo!()
                                                },
                                            }
                                            
                                            FreeRtos::delay_ms(self.delay_ms);
                                        }
                                        
                                        let average = values
                                            .iter()
                                            .sum::<u16>() as f32
                                            / (values.len() as f32);
                                        
                                        measurement.voltage = average * VOLTAGE_COEFICIENT;
                                        
                                        // DEBUG
                                        warn!("### average: {}mV * coef: {} -> {}",
                                              average,
                                              VOLTAGE_COEFICIENT,
                                              measurement.voltage,
                                        );
                                        
                                        // send measurement
                                        if let Err(_e) = self.sender.send(measurement) {
                                            // todo!()
                                        }
                                    },
                                }
                            }
                    }
                },
                Err(_e) => {
                    // todo!()
                },
            };
        });
    }
}

pub enum Command {
    Measure
}

#[derive(Debug)]
pub struct Measurement {
    pub voltage: f32,
}

impl Default for Measurement {
    //
    fn default() -> Self {
        Self {
            voltage: 0.0,
        }
    }
}

//
#[allow(unused)]
pub fn start_via_pin<PIN, ADC, const ATTN: u32>(
    gpio: PIN,
    adc_peripheral: ADC,
    delay_ms: u32,
    receiver: Receiver<Command>,
    sender: Sender<Measurement>
)
where
    ADC: Adc + Peripheral<P = ADC> + 'static, <ADC as Peripheral>::P: Adc,
    PIN: IOPin + ADCPin<Adc = ADC>,
{
    std::thread::spawn(move || {
        let pin_id = gpio.pin();
        let adc_channel_driver: Result<AdcChannelDriver::< ATTN, _>, esp_idf_sys::EspError> = AdcChannelDriver::new(gpio);

        match adc_channel_driver {
            Ok(mut cd) => {
                let adc_driver = AdcDriver::new(
                    adc_peripheral,
                    &adc::config::Config::new().calibration(true),
                );

                if let Ok(mut d) = adc_driver {
                    while let Ok(channel_data) = receiver.recv() {
                        match channel_data {
                            Command::Measure => {
                                let mut measurement = Measurement::default();
                                // todo!() via conf?
                                let mut counter = 0;
                                let mut values = Vec::new();
                                
                                while counter < REPETITION {
                                    counter += 1;
                                    
                                    match d.read(&mut cd) {
                                        Ok(value) => {
                                            // DEBUG
                                            warn!("$$$ PIN: {pin_id} [{counter:03}] {value} mV");
                                            values.push(value);
                                          
                                        },
                                        Err(_e) => {
                                            // todo!()
                                        },
                                    }
                                    
                                    FreeRtos::delay_ms(delay_ms);
                                }

                                let average = values
                                    .iter()
                                    .sum::<u16>() as f32
                                    / (values.len() as f32);
                                
                                measurement.voltage = average * VOLTAGE_COEFICIENT;
                               
                                // DEBUG
                                warn!("### average: {}mV * coef: {} -> {}",
                                      average,
                                      VOLTAGE_COEFICIENT,
                                      measurement.voltage,
                                );

                                // send measurement
                                if let Err(_e) = sender.send(measurement) {
                                    // todo!()
                                }
                            },
                        }
                    }
                }
            },
            Err(_e) => {
                // todo!()
            },
        };
    });
}
    
//
#[allow(unused)]
pub fn measure_pin<PIN, ADC, const ATTN: u32>(gpio: PIN,
                                              adc_peripheral: ADC,
) -> Result<(), esp_idf_sys::EspError>
where
    ADC: Adc + Peripheral<P = ADC>, <ADC as Peripheral>::P: Adc,
    PIN: esp_idf_hal::gpio::IOPin + ADCPin<Adc = ADC>,
{
    let pin_id = gpio.pin();

    let mut adc_channel_driver: AdcChannelDriver::< ATTN, _> = AdcChannelDriver::new(gpio)?;

    let mut adc_driver = AdcDriver::new(
        adc_peripheral,
        &adc::config::Config::new().calibration(true),
    )?;

    let mut measurement = Measurement::default();
    
    let mut counter = 0;
    let mut values = Vec::new();
    while counter < REPETITION {
        counter += 1;
        
        let value = adc_driver.read(&mut adc_channel_driver)?;

        warn!("$$$ PIN: {pin_id} [{counter:03}] {value} mV");
        values.push(value);
        
        FreeRtos::delay_ms(DELAY_MEASUREMENT_MS);
    }

    let average = values
        .iter()
        .sum::<u16>() as f32
        / (values.len() as f32);
    
    measurement.voltage = average * VOLTAGE_COEFICIENT;
    
    warn!("$$$ ADC -> average: {} mV / {}",
          average,
          measurement.voltage,
    );
    
    Ok(())
}

//
#[allow(unused)]
pub fn measure_channel_driver<const ATTN: u32, PIN, ADC>(
    adc_channel_driver: &mut AdcChannelDriver<ATTN, PIN>,
    adc_peripheral: ADC,
    //adc_peripheral: &mut ADC,
) -> Result<(), esp_idf_sys::EspError> 
where
    ADC: Adc + Peripheral<P = ADC>, <ADC as Peripheral>::P: Adc,
    PIN: ADCPin<Adc = ADC>,
{
    let mut adc_driver = AdcDriver::new(
        adc_peripheral,
        &adc::config::Config::new().calibration(true),
    )?;

    let mut measurement = Measurement::default();
    
    let mut counter = 0;
    let mut values = Vec::new();
    while counter < REPETITION {
        counter += 1;
        
        let value = adc_driver.read(adc_channel_driver)?;

        warn!("$$$ ADC?-?: [{counter:03}] {value} mV");
        values.push(value);
        
        FreeRtos::delay_ms(DELAY_MEASUREMENT_MS);
    }

    let average = values
        .iter()
        .sum::<u16>() as f32
        / (values.len() as f32);
    
    measurement.voltage = average * VOLTAGE_COEFICIENT;
    
    warn!("$$$ ADC -> average: {} mV / {}",
          average,
          measurement.voltage,
    );

    
    Ok(())
}
