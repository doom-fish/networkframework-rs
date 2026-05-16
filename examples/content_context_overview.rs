use networkframework::ContentContext;

fn main() -> Result<(), networkframework::NetworkError> {
    let antecedent = ContentContext::new("first")?;
    let mut context = ContentContext::new("second")?;
    context
        .set_is_final(true)
        .set_expiration_milliseconds(1_000)
        .set_relative_priority(0.75)
        .set_antecedent(Some(&antecedent));

    println!(
        "context={} final={} expiration={} priority={} antecedent={:?}",
        context.identifier(),
        context.is_final(),
        context.expiration_milliseconds(),
        context.relative_priority(),
        context.copy_antecedent().map(|value| value.identifier()),
    );
    Ok(())
}
