//! Example: generate MAC addresses in various modes.

use macrandom::{MacAddress, RandomizeMode};

fn main() {
    let local = MacAddress::random_local();
    println!(
        "random local:     {local}  (U/L={}, multicast={})",
        local.is_locally_administered(),
        local.is_multicast()
    );

    let orig: MacAddress = "00:1a:2b:33:44:55".parse().unwrap();
    let vendor = RandomizeMode::VendorPreserving.generate(&orig);
    println!(
        "vendor preserve:  {vendor}  (OUI={:02x}:{:02x}:{:02x})",
        vendor.oui()[0],
        vendor.oui()[1],
        vendor.oui()[2]
    );

    let full = RandomizeMode::FullyRandom.generate(&orig);
    println!("fully random:     {full}");
}
