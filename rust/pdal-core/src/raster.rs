//! Raster attachments carried by point views.

#[derive(Clone, Debug, PartialEq)]
pub struct RasterLimits {
    pub x_origin: f64,
    pub y_origin: f64,
    pub width: usize,
    pub height: usize,
    pub edge_length: f64,
}

impl RasterLimits {
    pub fn new(
        x_origin: f64,
        y_origin: f64,
        width: usize,
        height: usize,
        edge_length: f64,
    ) -> Self {
        Self {
            x_origin,
            y_origin,
            width,
            height,
            edge_length,
        }
    }

    pub fn len(&self) -> usize {
        self.width.saturating_mul(self.height)
    }

    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }
}

#[derive(Clone, Debug)]
pub struct RasterData {
    name: String,
    limits: RasterLimits,
    data: Vec<f64>,
    initializer: f64,
}

impl RasterData {
    pub fn new(name: impl Into<String>, limits: RasterLimits, initializer: f64) -> Self {
        Self {
            name: name.into(),
            data: vec![initializer; limits.len()],
            limits,
            initializer,
        }
    }

    pub fn from_data(
        name: impl Into<String>,
        limits: RasterLimits,
        data: Vec<f64>,
        initializer: f64,
    ) -> Result<Self, String> {
        if data.len() != limits.len() {
            return Err("Raster data length does not match raster limits.".to_string());
        }
        Ok(Self {
            name: name.into(),
            limits,
            data,
            initializer,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn limits(&self) -> &RasterLimits {
        &self.limits
    }

    pub fn data(&self) -> &[f64] {
        &self.data
    }

    pub fn initializer(&self) -> f64 {
        self.initializer
    }

    pub fn set_top_down(&mut self, row: usize, col: usize, value: f64) {
        let idx = row * self.limits.width + col;
        self.data[idx] = value;
    }
}
