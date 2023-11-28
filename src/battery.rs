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

type Voltage = f32;
type VoltageDividerCoeficient = f32;

#[derive(Debug)]
pub enum Status {
    Full,
    ToReplace,
    Init,
    Unknown
}

impl Status {
    //
    pub fn show(&self,
                msg: &str,
                measurement: &Measurement,
    ) {
        // try harder to format! dynamicky and not repeat !!!
        match self {
            Self::Full => {
                info!("{}: {:?}",
                      msg,
                      measurement,
                );
            },
            Self::ToReplace => {
                error!("{}: {:?}",
                       msg,
                       measurement,
                );
            },
            Self::Init => {
                warn!("{}: {:?}",
                      msg,
                      measurement,
                );
            },
            Self::Unknown => {
                warn!("verify battery and cables, value is too low!!! \n{}  {:?}",
                      msg,
                      measurement,
                );
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[allow(unused)]
pub struct Property {
    voltage_expected: Voltage,
    voltage_coeficient: VoltageDividerCoeficient,
    voltage_warning_boundary: Voltage,
}

impl Property {
    //
    pub fn new(voltage_expected: Voltage,
               voltage_coeficient: VoltageDividerCoeficient,
               voltage_warning_boundary: Voltage,
    ) -> Self {
        Self {
            voltage_expected,
            voltage_coeficient,
            voltage_warning_boundary,
        }
    }

    //
    fn get_boundary(&self) -> Voltage {
        self.voltage_warning_boundary
    }

    //
    fn get_coeficient(&self) -> VoltageDividerCoeficient {
        self.voltage_coeficient
    }
}
    
// for periodic measuring
//
// https://users.rust-lang.org/t/how-to-store-a-trait-as-field-of-a-struct/87762/2
//
/*
!!!
pub struct Sensor<'a, PIN, ADC, const ATTN: u32> {
!!! why do i need to have PIN: ADCPin when already via WHERE ??? study more !!!
pub struct Sensor<'a, PIN: ADCPin, ADC, const ATTN: u32> {
!!!

error[E0277]: the trait bound `PIN: ADCPin` is not satisfied
   --> src/battery.rs:30:25
    |
30  |     adc_channel_driver: AdcChannelDriver<'a, ATTN, PIN>,
    |                         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ the trait `ADCPin` is not implemented for `PIN`
    |
note: required by a bound in `AdcChannelDriver`
   --> /home/conan/.cargo/registry/src/index.crates.io-6f17d22bba15001f/esp-idf-hal-0.42.5/src/adc.rs:123:58
    |
123 | pub struct AdcChannelDriver<'d, const A: adc_atten_t, T: ADCPin> {
    |                                                          ^^^^^^ required by this bound in `AdcChannelDriver`
help: consider restricting type parameter `PIN`
    |
27  | pub struct Sensor<'a, PIN: esp_idf_hal::gpio::ADCPin, ADC, const ATTN: u32> {
    |                          +++++++++++++++++++++++++++

For more information about this error, try `rustc --explain E0277`.
*/
pub struct Sensor<'a, PIN: ADCPin, ADC, const ATTN: u32> {
    pin_id: i32,
    adc_channel_driver: AdcChannelDriver<'a, ATTN, PIN>,
    adc_peripheral: Arc<Mutex<ADC>>,
    sender: Sender<Measurement>,
    property: Property,
}

impl<PIN, ADC, const ATTN: u32> Sensor<'_, PIN, ADC, ATTN>
where
    PIN: esp_idf_hal::gpio::IOPin + ADCPin<Adc = ADC>,
    ADC: Adc + Peripheral<P = ADC>, <ADC as Peripheral>::P: Adc,
{

    //
    pub fn new(gpio: PIN,
               adc_peripheral: Arc<Mutex<ADC>>,
               sender: Sender<Measurement>,
               property: Property,
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
                 property,
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
                         self.property,
                )?;
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
    property: Property,
    voltage: Voltage,
    raw_u16: u16,
    raw_f32: f32,
    attn: u32,
    status: Status,
}

impl Measurement {
    //
    fn new(pin_id: i32,
           property: Property,
           voltage: Voltage,
           raw_u16: u16,
           raw_f32: f32,
           attn: u32,
    ) -> Self {
        Self {
            pin_id,
            property,
            voltage,
            raw_u16,
            raw_f32,
            attn,
            status: Status::Init,
        }
    }
    
    //
    fn get_voltage(&self) -> Voltage {
        self.voltage
    }

    //
    fn verify_boundary(&mut self) {
        let voltage = self.get_voltage();
        let boundary = self.property.get_boundary();

        self.status =
            // 27.335 < (3500/2) [half of boundary]
            // as raw_u16: 5 and raw_f32: 5.1 * coeficient: 4.97 = 2.335
            if voltage < (boundary/2.0) {
                Status::Unknown
            // 3300 < 3500
            } else if voltage < boundary {
                Status::ToReplace
            // 4138
            } else {
                Status::Full
            };
    }

    //
    pub fn get_status(&self,
                      msg: &str,
    ) {
        self.status.show(&msg,
                         self,);
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
    property: Property,
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
                                              property,
                                              ATTN,
    );

    // send measurement
    if let Err(e) = sender.send(measurement) {
        error!("Error: sender .send(measurement) -> {e:?}");
    }
    
    Ok(())
}

//
fn calculate_measured_data(pin_id: i32,
                           values: Vec<u16>,
                           property: Property,
                           attn: u32,
) -> Measurement {
    let average: Voltage = values
        .iter()
        .sum::<u16>() as f32
        / (values.len() as f32);

    let mut measurement = Measurement::new(
        pin_id,
        property,
        average * property.get_coeficient(),
        average as u16,
        average,
        attn,
    );
    
    measurement.verify_boundary();
    
    // DEBUG
    info!("$$$ ADC -> average: {} mV / {}V",
          average,
          measurement.voltage,
    );

    measurement
}

//
// just one measurement (without any struct) and then we can go deepsleep
//
// this cannot be used in std::thread::spawn as it will reboot machine!!!
//
pub fn measure_pin_once<PIN, ADC, const ATTN: u32, D>(
    gpio: Arc<Mutex<PIN>>,
    adc_peripheral: Arc<Mutex<ADC>>,
    sender: Sender<Measurement>,
    property: Property,
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
                             property,
                    )?;
                },
                Err(_e) => {},
            }
        },
        Err(_e) => {},
    }
    
    Ok(())
}

//
// just one measurement (without any struct) and then we can go deepsleep
//
// this can be used in std::thread::spawn
//
pub fn measure_channel_driver_once<const ATTN: u32, PIN, ADC, D>(
    pin_id: i32,
    adc_channel_driver: &mut AdcChannelDriver<ATTN, PIN>,
    adc_peripheral: Arc<Mutex<ADC>>,
    sender: Sender<Measurement>,
    property: Property,
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

            read_adc(adc_channel_driver,
                     adc_driver,
                     pin_id,
                     delay,
                     sender,
                     property,
            )?;
        },
        Err(_e) => {},
    }
    
    Ok(())
}
