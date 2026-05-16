use networkframework::{
    ConnectionParameters, InterfaceType, ParametersAttribution, ProtocolOptions,
};

fn main() -> Result<(), networkframework::NetworkError> {
    let mut parameters = ConnectionParameters::generic()?;
    parameters
        .set_attribution(ParametersAttribution::User)
        .set_required_interface_type(InterfaceType::Loopback)
        .set_prohibit_expensive(true)
        .set_prohibit_constrained(true)
        .set_allow_ultra_constrained(true)
        .set_prefer_no_proxy(true);
    parameters.prepend_application_protocol(&ProtocolOptions::websocket()?)?;

    println!(
        "attribution={:?} interface={:?} expensive={} constrained={} ultra_constrained={}",
        parameters.attribution(),
        parameters.required_interface_type(),
        parameters.prohibit_expensive(),
        parameters.prohibit_constrained(),
        parameters.allow_ultra_constrained(),
    );

    if let Ok(application_service) = ConnectionParameters::application_service() {
        let _ = application_service;
        println!("application-service parameters are supported on this system");
    }

    Ok(())
}
