use std::io::Cursor;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use image::{DynamicImage, GenericImageView, ImageFormat, ImageReader};

const MAX_SOURCE_BYTES: usize = 8 * 1024 * 1024;
const MENU_ICON_MAX_SIZE: u32 = 32;
const PNG_DATA_URL_PREFIX: &str = "data:image/png;base64,";

pub fn normalize_data_url(data_url: &str) -> Result<String, String> {
    let image = decode(data_url)?;
    let resized = fit_menu_size(image);
    let mut bytes = Cursor::new(Vec::new());
    resized
        .write_to(&mut bytes, ImageFormat::Png)
        .map_err(|error| format!("PNG 编码失败：{error}"))?;
    Ok(format!(
        "{PNG_DATA_URL_PREFIX}{}",
        STANDARD.encode(bytes.into_inner())
    ))
}

pub fn decode_menu_image(data_url: &str) -> Result<tauri::image::Image<'static>, String> {
    let rgba = fit_menu_size(decode(data_url)?).into_rgba8();
    let (width, height) = rgba.dimensions();
    Ok(tauri::image::Image::new_owned(
        rgba.into_raw(),
        width,
        height,
    ))
}

fn fit_menu_size(image: DynamicImage) -> DynamicImage {
    let (width, height) = image.dimensions();
    if width > MENU_ICON_MAX_SIZE || height > MENU_ICON_MAX_SIZE {
        image.thumbnail(MENU_ICON_MAX_SIZE, MENU_ICON_MAX_SIZE)
    } else {
        image
    }
}

fn decode(data_url: &str) -> Result<DynamicImage, String> {
    let (metadata, encoded) = data_url
        .split_once(',')
        .ok_or_else(|| "图标必须是 Data URL".to_string())?;
    if !metadata.starts_with("data:image/") || !metadata.ends_with(";base64") {
        return Err("图标必须是 Base64 图片 Data URL".to_string());
    }
    let bytes = STANDARD
        .decode(encoded)
        .map_err(|error| format!("Base64 解码失败：{error}"))?;
    if bytes.len() > MAX_SOURCE_BYTES {
        return Err("图标文件不能超过 8 MiB".to_string());
    }
    ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|error| format!("无法识别图片格式：{error}"))?
        .decode()
        .map_err(|error| format!("图片解码失败：{error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};

    fn test_data_url(width: u32, height: u32) -> String {
        let image = DynamicImage::ImageRgba8(ImageBuffer::from_pixel(
            width,
            height,
            Rgba([32, 64, 128, 128]),
        ));
        let mut bytes = Cursor::new(Vec::new());
        image.write_to(&mut bytes, ImageFormat::Png).unwrap();
        format!(
            "data:image/png;base64,{}",
            STANDARD.encode(bytes.into_inner())
        )
    }

    #[test]
    fn normalizes_to_png_and_preserves_aspect_ratio() {
        let normalized = normalize_data_url(&test_data_url(128, 64)).unwrap();
        assert!(normalized.starts_with(PNG_DATA_URL_PREFIX));
        let decoded = decode(&normalized).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (32, 16));
    }

    #[test]
    fn produces_owned_rgba_for_tauri_menu() {
        let image = decode_menu_image(&test_data_url(16, 24)).unwrap();
        assert_eq!((image.width(), image.height()), (16, 24));
        assert_eq!(image.rgba().len(), 16 * 24 * 4);
    }

    #[test]
    fn rejects_invalid_data_url_without_panicking() {
        assert!(normalize_data_url("data:image/png;base64,not-valid").is_err());
        assert!(normalize_data_url("/Users/example/icon.png").is_err());
    }
}
