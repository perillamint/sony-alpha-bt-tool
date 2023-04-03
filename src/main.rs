use std::time::Duration;

use btleplug::api::{bleuuid::uuid_from_u16, Central, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::{Adapter, Manager, Peripheral};
use byteorder::{BigEndian, ByteOrder};
use chrono::{Local, Timelike, Datelike, DateTime};
use tokio::time;
use uuid::{Uuid, uuid};

const CONTROL_SERVICE_UUID: Uuid = uuid!("8000CC00-CC00-FFFF-FFFF-FFFFFFFFFFFF");

const LOCATION_SERVICE_UUID: Uuid = uuid!("8000DD00-DD00-FFFF-FFFF-FFFFFFFFFFFF");
const LOCATION_CHARACTERISTIC_NOTIFY_UUID: Uuid = uuid_from_u16(0xDD01);
const LOCATION_CHARACTERISTIC_INFO_UUID: Uuid = uuid_from_u16(0xDD11);
const LOCATION_CHARACTERISTIC_FEATURE_UUID: Uuid = uuid_from_u16(0xDD21);

const PAIRING_SERVICE_UUID: Uuid = uuid!("8000EE00-EE00-FFFF-FFFF-FFFFFFFFFFFF");
const PAIRING_CHARACTERISTIC_UUID: Uuid = uuid_from_u16(0xEE01);

const REMOTE_CONTROL_SERVICE_UUID: Uuid = uuid!("8000FF00-FF00-FFFF-FFFF-FFFFFFFFFFFF");
const REMOTE_CONTROL_CHARACTERISTIC_COMMAND_UUID: Uuid = uuid_from_u16(0xFF01);
const REMOTE_CONTROL_CHARACTERISTIC_NOTIFY_UUID: Uuid = uuid_from_u16(0xFF02);

fn location_payload(lat: f64, lng: f64, now: DateTime<Local>) -> Vec<u8> {
    // Payload format reference: https://github.com/whc2001/ILCE7M3ExternalGps/blob/main/PROTOCOL_EN.md
    let mut data: Vec<u8> = vec![0; 95];

    BigEndian::write_u16(&mut data[0..2], 0x005d);
    BigEndian::write_u24(&mut data[2..5], 0x0802fc);

    // Report type: With timezone offset and DST.
    data[5] = 0x03;

    data[6] = 0x00;
    BigEndian::write_u32(&mut data[7..11], 0x0010_1010);

    BigEndian::write_u32(&mut data[11..15], (lat * (10000000 as f64)) as u32);
    BigEndian::write_u32(&mut data[15..19], (lng * (10000000 as f64)) as u32);

    BigEndian::write_u16(&mut data[19..21], now.year() as u16);
    data[21] = now.month() as u8;
    data[22] = now.day() as u8;
    data[23] = now.hour() as u8;
    data[24] = now.minute() as u8;
    data[25] = now.second() as u8;

    // Zeros from 26 to 90

    BigEndian::write_u16(&mut data[91..93], (now.offset().local_minus_utc() / 60) as u16);
    BigEndian::write_u16(&mut data[93..95], 0x0000); // DST offset in munute

    data
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let manager = Manager::new().await.unwrap();

    // get the first bluetooth adapter
    let adapters = manager.adapters().await?;
    let central = adapters.into_iter().nth(0).unwrap();

    // start scanning for devices
    central.start_scan(ScanFilter::default()).await?;
    // instead of waiting, you can use central.events() to get a stream which will
    // notify you of new devices, for an example of that see examples/event_driven_discovery.rs
    time::sleep(Duration::from_secs(2)).await;
 
    // find the device we're interested in
    let camera = find_light(&central).await.unwrap();
 
    // connect to the device
    camera.connect().await?;
 
    // discover services and characteristics
    camera.discover_services().await?;

    // find the characteristic we want
    let chars = camera.characteristics();
    let location_feature = chars.iter().find(|c| c.uuid == LOCATION_CHARACTERISTIC_INFO_UUID).unwrap();
    println!("{:#?}", location_feature);

    let now = Local::now();
    let data: Vec<u8> = location_payload(27.986065, 86.922623, now);

    camera.write(location_feature, &data, WriteType::WithResponse).await?;

    Ok(())
}

async fn find_light(central: &Adapter) -> Option<Peripheral> {
    for p in central.peripherals().await.unwrap() {
        if p.properties()
            .await
            .unwrap()
            .unwrap()
            .local_name
            .iter()
            .any(|name| name.contains("ILCE-7C"))
        {
            return Some(p);
        }
    }
    None
}