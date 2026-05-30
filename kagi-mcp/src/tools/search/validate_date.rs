use rmcp::ErrorData;

pub fn validate_date(value: &Option<String>, field_name: &str) -> Result<(), ErrorData> {
    let Some(date) = value.as_deref() else {
        return Ok(());
    };
    if date.len() != 10 {
        return Err(ErrorData::invalid_params(
            format!("{field_name} must match YYYY-MM-DD format"),
            None,
        ));
    }
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 || parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
        return Err(ErrorData::invalid_params(
            format!("{field_name} must match YYYY-MM-DD format"),
            None,
        ));
    }
    let month: u32 = parts[1].parse().map_err(|_| {
        ErrorData::invalid_params(format!("{field_name} must match YYYY-MM-DD format"), None)
    })?;
    let day: u32 = parts[2].parse().map_err(|_| {
        ErrorData::invalid_params(format!("{field_name} must match YYYY-MM-DD format"), None)
    })?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return Err(ErrorData::invalid_params(
            format!("{field_name} must match YYYY-MM-DD format"),
            None,
        ));
    }
    Ok(())
}
