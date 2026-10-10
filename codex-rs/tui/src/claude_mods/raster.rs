use base64::Engine;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;

pub(super) fn lines(
    cells: &str,
    columns: usize,
    rows: usize,
) -> Result<Vec<Line<'static>>, String> {
    if columns == 0 || columns > 512 || rows > 256 {
        return Err("invalid mod raster dimensions".into());
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(cells)
        .map_err(|error| error.to_string())?;
    let expected = columns
        .checked_mul(rows)
        .and_then(|size| size.checked_mul(12))
        .ok_or_else(|| "mod raster dimensions overflow".to_string())?;
    if bytes.len() != expected {
        return Err(format!(
            "mod raster has {} bytes; expected {expected}",
            bytes.len()
        ));
    }
    let mut lines = Vec::with_capacity(rows);
    for row in bytes.chunks_exact(columns * 12) {
        let mut spans = Vec::with_capacity(columns);
        for cell in row.chunks_exact(12) {
            let codepoint = u32::from_le_bytes(
                cell[0..4]
                    .try_into()
                    .map_err(|error: std::array::TryFromSliceError| error.to_string())?,
            );
            let foreground = u32::from_le_bytes(
                cell[4..8]
                    .try_into()
                    .map_err(|error: std::array::TryFromSliceError| error.to_string())?,
            );
            let background = u32::from_le_bytes(
                cell[8..12]
                    .try_into()
                    .map_err(|error: std::array::TryFromSliceError| error.to_string())?,
            );
            let symbol = char::from_u32(codepoint)
                .filter(|character| !character.is_control())
                .unwrap_or(' ');
            let style = Style::default()
                .fg(color(foreground)?)
                .bg(color(background)?);
            spans.push(Span::styled(symbol.to_string(), style));
        }
        lines.push(Line::from(spans));
    }
    Ok(lines)
}

fn color(value: u32) -> Result<Color, String> {
    if value == 0x01000000 {
        Ok(Color::Reset)
    } else {
        format!("#{:06x}", value & 0xffffff)
            .parse::<Color>()
            .map_err(|error| error.to_string())
    }
}
