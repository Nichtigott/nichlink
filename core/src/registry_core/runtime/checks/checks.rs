//! Runtime value checks.
//! 运行时值检查。

use super::{RuntimeCheckFailure, RuntimeValue};

pub(super) fn split_items(value: &str) -> Vec<&str> {
    let mut items = Vec::new();
    let mut start = 0;
    let mut depth = 0usize;
    for (index, character) in value.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                items.push(value[start..index].trim());
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    items.push(value[start..].trim());
    items
}

fn wrong_kind(check: &'static str, expected: &str, value: &RuntimeValue) -> RuntimeCheckFailure {
    RuntimeCheckFailure {
        check,
        message: format!("expected {expected} data, received {}", value.kind()),
        provenance: value.provenance().clone(),
    }
}

pub(super) fn coordinates(value: &RuntimeValue) -> Result<(), RuntimeCheckFailure> {
    let RuntimeValue::Coordinates(coordinates) = value else {
        return Err(wrong_kind("coordinates_in_viewport", "coordinate", value));
    };
    let finite = [
        coordinates.x,
        coordinates.y,
        coordinates.width,
        coordinates.height,
        coordinates.viewport_width,
        coordinates.viewport_height,
    ]
    .into_iter()
    .all(f32::is_finite);
    if !finite {
        return Err(RuntimeCheckFailure {
            check: "coordinates_in_viewport",
            message: "coordinate contains NaN or infinity".to_owned(),
            provenance: coordinates.provenance.clone(),
        });
    }
    if coordinates.coordinate_space != coordinates.expected_space {
        return Err(RuntimeCheckFailure {
            check: "coordinates_in_viewport",
            message: format!(
                "coordinate space `{}` does not match expected `{}`",
                coordinates.coordinate_space, coordinates.expected_space
            ),
            provenance: coordinates.provenance.clone(),
        });
    }
    let inside = coordinates.x >= 0.0
        && coordinates.y >= 0.0
        && coordinates.width >= 0.0
        && coordinates.height >= 0.0
        && coordinates.x + coordinates.width <= coordinates.viewport_width
        && coordinates.y + coordinates.height <= coordinates.viewport_height;
    if inside {
        Ok(())
    } else {
        Err(RuntimeCheckFailure {
            check: "coordinates_in_viewport",
            message: format!(
                "rect ({:.1}, {:.1}, {:.1}, {:.1}) exceeds viewport ({:.1}, {:.1})",
                coordinates.x,
                coordinates.y,
                coordinates.width,
                coordinates.height,
                coordinates.viewport_width,
                coordinates.viewport_height
            ),
            provenance: coordinates.provenance.clone(),
        })
    }
}

pub(super) fn finite_number(value: &RuntimeValue) -> Result<(), RuntimeCheckFailure> {
    let RuntimeValue::Number { value, provenance } = value else {
        return Err(wrong_kind("finite_number", "number", value));
    };
    if value.is_finite() {
        Ok(())
    } else {
        Err(RuntimeCheckFailure {
            check: "finite_number",
            message: format!("number `{value}` is NaN or infinite"),
            provenance: provenance.clone(),
        })
    }
}

pub(super) fn number_range(
    value: &RuntimeValue,
    min: i64,
    max: i64,
) -> Result<(), RuntimeCheckFailure> {
    let RuntimeValue::Number { value, provenance } = value else {
        return Err(wrong_kind("number_in_range", "number", value));
    };
    if min > max {
        return Err(RuntimeCheckFailure {
            check: "number_in_range",
            message: format!("invalid check bounds: minimum {min} exceeds maximum {max}"),
            provenance: provenance.clone(),
        });
    }
    if value.is_finite() && *value >= min as f64 && *value <= max as f64 {
        Ok(())
    } else {
        Err(RuntimeCheckFailure {
            check: "number_in_range",
            message: format!("number `{value}` is outside inclusive range {min}..={max}"),
            provenance: provenance.clone(),
        })
    }
}

pub(super) fn non_empty_text(value: &RuntimeValue) -> Result<(), RuntimeCheckFailure> {
    let RuntimeValue::Text { value, provenance } = value else {
        return Err(wrong_kind("non_empty_text", "text", value));
    };
    if value.trim().is_empty() {
        Err(RuntimeCheckFailure {
            check: "non_empty_text",
            message: "text is empty or whitespace only".to_owned(),
            provenance: provenance.clone(),
        })
    } else {
        Ok(())
    }
}

pub(super) fn text_length(
    value: &RuntimeValue,
    min: usize,
    max: usize,
) -> Result<(), RuntimeCheckFailure> {
    let RuntimeValue::Text { value, provenance } = value else {
        return Err(wrong_kind("text_length", "text", value));
    };
    let length = value.chars().count();
    if min <= max && (min..=max).contains(&length) {
        Ok(())
    } else {
        Err(RuntimeCheckFailure {
            check: "text_length",
            message: if min > max {
                format!("invalid check bounds: minimum {min} exceeds maximum {max}")
            } else {
                format!("text length {length} is outside inclusive range {min}..={max}")
            },
            provenance: provenance.clone(),
        })
    }
}
