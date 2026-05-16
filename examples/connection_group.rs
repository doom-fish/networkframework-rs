use networkframework::{
    ConnectionGroup, ConnectionGroupDescriptor, ConnectionGroupState, ConnectionParameters,
};
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn main() -> Result<(), networkframework::NetworkError> {
    let descriptor = ConnectionGroupDescriptor::multicast("239.255.0.1", 5000)?;
    let parameters = ConnectionParameters::udp()?;
    let mut group = ConnectionGroup::new(&descriptor, &parameters)?;

    let states = Arc::new(Mutex::new(Vec::new()));
    let states_for_callback = Arc::clone(&states);
    group.set_state_changed_handler(move |state| {
        println!("state: {state:?}");
        states_for_callback.lock().expect("state lock").push(state);
    });
    group.set_receive_handler(2048, false, |_message| {});
    group.start()?;
    std::thread::sleep(Duration::from_millis(200));

    let observed_states = states.lock().expect("state lock");
    assert!(!observed_states.is_empty());
    assert!(observed_states.iter().any(|state| matches!(
        state,
        ConnectionGroupState::Ready | ConnectionGroupState::Waiting
    )));
    println!(
        "connection group started with {} observed state update(s)",
        observed_states.len()
    );
    drop(observed_states);

    group.cancel();
    std::thread::sleep(Duration::from_millis(100));
    Ok(())
}
