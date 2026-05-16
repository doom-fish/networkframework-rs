use networkframework::list_interfaces;

fn main() {
    let interfaces = list_interfaces();
    println!("found {} interfaces", interfaces.len());
    for interface in interfaces {
        println!(
            "{} {:?} #{}",
            interface.name, interface.interface_type, interface.index
        );
    }
}
