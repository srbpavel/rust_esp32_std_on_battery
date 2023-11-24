#[allow(unused_imports)]
use log::error;
#[allow(unused_imports)]
use log::info;
#[allow(unused_imports)]
use log::warn;

use embedded_hal::blocking::delay::DelayMs;

use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::sync::Mutex;

use esp_idf_hal::gpio::ADCPin;
use esp_idf_hal::peripheral::Peripheral;

use esp_idf_hal::adc;
use esp_idf_hal::adc::Adc;
use esp_idf_hal::adc::AdcChannelDriver;
use esp_idf_hal::adc::AdcDriver;

// for periodic measuring
//
// https://users.rust-lang.org/t/how-to-store-a-trait-as-field-of-a-struct/87762/2
//
pub struct Sensor<'a, PIN: ADCPin, ADC, const ATTN: u32> {
//pub struct Sensor<'a, 'b, PIN: ADCPin, ADC, const ATTN: u32, D> {
    pin_id: i32,
    adc_channel_driver: AdcChannelDriver<'a, ATTN, PIN>,
    adc_peripheral: Arc<Mutex<ADC>>,
    sender: Sender<Measurement>,
    voltage_coeficient: f32,
    //delay: &'b mut D,
    battery_warning_boundary: f32,
}

impl<PIN, ADC, const ATTN: u32> Sensor<'_, PIN, ADC, ATTN>
//impl<PIN, ADC, const ATTN: u32, D> Sensor<'_, '_, PIN, ADC, ATTN, D>
where
    PIN: esp_idf_hal::gpio::IOPin + ADCPin<Adc = ADC>,
    ADC: Adc + Peripheral<P = ADC>, <ADC as Peripheral>::P: Adc,
    //D: embedded_hal::blocking::delay::DelayMs<u32> + std::marker::Send + 'static,
{

    //
    pub fn new(gpio: PIN,
               adc_peripheral: Arc<Mutex<ADC>>,
               sender: Sender<Measurement>,
               voltage_coeficient: f32,
               //delay: &mut D,
               battery_warning_boundary: f32,
    ) -> Result<Self, esp_idf_sys::EspError>
     {
         let pin_id = gpio.pin();
         
         let adc_channel_driver: AdcChannelDriver::<ATTN, _> = AdcChannelDriver::new(gpio)?;
         
         Ok(
             Self {
                 pin_id,
                 adc_channel_driver,
                 adc_peripheral,
                 sender,
                 voltage_coeficient,
                 battery_warning_boundary,
             }
         )
     }
    
    //
    pub fn measure<D>(&mut self,
                      delay: &mut D,
    ) -> Result<(), esp_idf_sys::EspError>
    where
        D: DelayMs<u32> + std::marker::Send + 'static,
    {
        match self.adc_peripheral.lock() {
            Ok(adc_peripheral) => {
                let adc_driver = AdcDriver::new(
                    adc_peripheral,
                    &adc::config::Config::new().calibration(true),
                )?;

                read_adc(&mut self.adc_channel_driver,
                         adc_driver,
                         self.pin_id,
                         delay,
                         self.sender.clone(),
                         self.voltage_coeficient,
                         self.battery_warning_boundary,
                )?;
                
                /*
                let values = read_adc(&mut self.adc_channel_driver,
                                      adc_driver,
                                      self.pin_id,
                                      delay,
                )?;
                
                let measurement = calculate_measured_data(self.pin_id,
                                                          values,
                                                          self.voltage_coeficient,
                );

                if measurement.get_voltage() < self.battery_warning_boundary {
                    error!("BATTERY too low, replace with new !!!");
                }
                
                // send measurement
                if let Err(e) = self.sender.send(measurement) {
                    error!("Error: sender .send(measurement) -> {e:?}");
                }
                */
            },
            Err(_e) => {},
        }
        
        Ok(())
    }
}

#[derive(Debug)]
pub enum Command {
    Measure(i32)
}

#[derive(Debug)]
#[allow(unused)]
pub struct Measurement {
    pin_id: i32,
    voltage: f32,
    voltage_coeficient: f32,
    raw_u16: u16,
    raw_f32: f32,
}

impl Measurement {
    //
    fn new(pin_id: i32,
           voltage: f32,
           voltage_coeficient: f32,
           raw_u16: u16,
           raw_f32: f32,
    ) -> Self {
        Self {
            pin_id,
            voltage,
            voltage_coeficient,
            raw_u16,
            raw_f32,
        }
    }
    
    //
    pub fn get_voltage(&self) -> f32 {
        self.voltage
    }
}

//
// helper fn to have it only on one place
//
fn read_adc<'a, PIN: ADCPin, ADC, const ATTN: u32, D>(
    adc_channel_driver: &mut AdcChannelDriver<'a, ATTN, PIN>,
    mut adc_driver: esp_idf_hal::adc::AdcDriver<ADC>,
    pin_id: i32,
    delay: &mut D,
    sender: Sender<Measurement>,
    voltage_coeficient: f32,
    battery_warning_boundary: f32,
//) -> Result<Vec<u16>, esp_idf_sys::EspError>
) -> Result<(), esp_idf_sys::EspError>
where
    PIN: esp_idf_hal::gpio::IOPin + ADCPin<Adc = ADC>,
    ADC: Adc + Peripheral<P = ADC>, <ADC as Peripheral>::P: Adc,
    D: DelayMs<u32> + std::marker::Send + 'static,
{
    let mut counter = 0;
    let mut values = Vec::new();

    while counter < crate::ADC_READ_REPETITION {
        counter += 1;
        
        let value = adc_driver.read(adc_channel_driver)?;
        
        // DEBUG
        warn!("$$$ PIN: {} [{:03}] {value} mV",
              pin_id,
              counter,
        );
        
        values.push(value);
        
        delay.delay_ms(crate::DELAY_MEASUREMENT_MS);
    }

    let measurement = calculate_measured_data(pin_id,
                                              values,
                                              voltage_coeficient,
    );
    
    if measurement.get_voltage() < battery_warning_boundary {
        error!("BATTERY too low, replace with new !!!");
    }
    
    // send measurement
    if let Err(e) = sender.send(measurement) {
        error!("Error: sender .send(measurement) -> {e:?}");
    }
    
    //Ok(values)
    Ok(())
}

/*
//
// helper fn to have it only on one place
//
fn read_adc<'a, PIN: ADCPin, ADC, const ATTN: u32, D>(
    adc_channel_driver: &mut AdcChannelDriver<'a, ATTN, PIN>,
    mut adc_driver: esp_idf_hal::adc::AdcDriver<ADC>,
    pin_id: i32,
    delay: &mut D,
) -> Result<Vec<u16>, esp_idf_sys::EspError>
where
    PIN: esp_idf_hal::gpio::IOPin + ADCPin<Adc = ADC>,
    ADC: Adc + Peripheral<P = ADC>, <ADC as Peripheral>::P: Adc,
    D: DelayMs<u32> + std::marker::Send + 'static,
{
    let mut counter = 0;
    let mut values = Vec::new();

    while counter < crate::ADC_READ_REPETITION {
        counter += 1;
        
        let value = adc_driver.read(adc_channel_driver)?;
        
        // DEBUG
        warn!("$$$ PIN: {} [{:03}] {value} mV",
              pin_id,
              counter,
        );
        
        values.push(value);
        
        delay.delay_ms(crate::DELAY_MEASUREMENT_MS);
    }

    Ok(values)
}
*/

//
fn calculate_measured_data(pin_id: i32,
                           values: Vec<u16>,
                           voltage_coeficient: f32,
) -> Measurement {
    let average: f32 = values
        .iter()
        .sum::<u16>() as f32
        / (values.len() as f32);

    let measurement = Measurement::new(
        pin_id,
        average * voltage_coeficient,
        voltage_coeficient,
        average as u16,
        average,
    );
    
    // DEBUG
    warn!("$$$ ADC -> average: {} mV / {}",
          average,
          measurement.voltage,
    );

    measurement
}

//
// just one measurement (without any struct) and then we can go deepsleep or ...
//
// this cannot be used in std::thread::spawn as it will panic!!!
//
pub fn measure_pin_once<PIN, ADC, const ATTN: u32, D>(
    gpio: Arc<Mutex<PIN>>,
    adc_peripheral: Arc<Mutex<ADC>>,
    sender: Sender<Measurement>,
    voltage_coeficient: f32,
    battery_warning_boundary: f32,
    delay: &mut D,
) -> Result<(), esp_idf_sys::EspError>
where
    ADC: Adc + Peripheral<P = ADC>, <ADC as Peripheral>::P: Adc,
    PIN: esp_idf_hal::gpio::IOPin + ADCPin<Adc = ADC>,
    D: DelayMs<u32> + std::marker::Send + 'static,
{
    match gpio.lock() {
        Ok(gpio) => {
            let pin_id = gpio.pin();
            
            let mut adc_channel_driver: AdcChannelDriver::< ATTN, _> = AdcChannelDriver::new(gpio)?;

            match adc_peripheral.lock() {
                Ok(adc_peripheral) => {
                    let adc_driver = AdcDriver::new(
                        adc_peripheral,
                        &adc::config::Config::new().calibration(true),
                    )?;

                    read_adc(&mut adc_channel_driver,
                             adc_driver,
                             pin_id,
                             delay,
                             sender,
                             voltage_coeficient,
                             battery_warning_boundary,
                    )?;
                    
                    /*
                    let values = read_adc(&mut adc_channel_driver,
                                          adc_driver,
                                          pin_id,
                                          delay,
                    )?;
                    
                    let measurement = calculate_measured_data(pin_id,
                                                              values,
                                                              voltage_coeficient,
                    );

                    if measurement.get_voltage() < battery_warning_boundary {
                        error!("BATTERY too low, replace with new !!!");
                    }
                    
                    // send measurement
                    if let Err(e) = sender.send(measurement) {
                        error!("Error: sender .send(measurement) -> {e:?}");
                    }
                    */
                },
                Err(_e) => {},
            }
        },
        Err(_e) => {},
    }
    
    Ok(())
}

//
// just one measurement (without any struct) and then we can go deepsleep or ...
//
// this can be used in std::thread::spawn
//
pub fn measure_channel_driver_once<const ATTN: u32, PIN, ADC, D>(
    pin_id: i32,
    adc_channel_driver: &mut AdcChannelDriver<ATTN, PIN>,
    adc_peripheral: Arc<Mutex<ADC>>,
    sender: Sender<Measurement>,
    voltage_coeficient: f32,
    battery_warning_boundary: f32,
    delay: &mut D,
) -> Result<(), esp_idf_sys::EspError> 
where
    ADC: Adc + Peripheral<P = ADC>, <ADC as Peripheral>::P: Adc,
    PIN: esp_idf_hal::gpio::IOPin + ADCPin<Adc = ADC>,
    D: DelayMs<u32> + std::marker::Send + 'static,
{
    match adc_peripheral.lock() {
        Ok(adc_peripheral) => {
            let adc_driver = AdcDriver::new(
                adc_peripheral,
                &adc::config::Config::new().calibration(true),
            )?;

            //read_adc(&mut adc_channel_driver,
            read_adc(adc_channel_driver,
                     adc_driver,
                     pin_id,
                     delay,
                     sender,
                     voltage_coeficient,
                     battery_warning_boundary,
            )?;
            
            /*
            let values = read_adc(adc_channel_driver,
                                  adc_driver,
                                  pin_id,
                                  delay,
            )?;
            
            let measurement = calculate_measured_data(pin_id,
                                                      values,
                                                      voltage_coeficient,
            );
            
            if measurement.get_voltage() < battery_warning_boundary {
                error!("BATTERY too low, replace with new !!!");
            }

            // send measurement
            if let Err(e) = sender.send(measurement) {
                error!("Error: sender .send(measurement) -> {e:?}");
            }
            */
        },
        Err(_e) => {},
    }
    
    Ok(())
}

/*
//
fn start_calculation() {
    let values = read_adc(&mut adc_channel_driver,
                          adc_driver,
                          pin_id,
                          delay,
    )?;
    
    let measurement = calculate_measured_data(pin_id,
                                              values,
                                              voltage_coeficient,
    );
    
    if measurement.get_voltage() < battery_warning_boundary {
        error!("BATTERY too low, replace with new !!!");
    }
    
    // send measurement
    if let Err(e) = sender.send(measurement) {
        error!("Error: sender .send(measurement) -> {e:?}");
    }
}
*/
