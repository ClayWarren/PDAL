use pdal_core::options::Options;
use pdal_core::point::{DimId, PointId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

const GPS_TIME_OFFSET: f64 = 1_000_000_000.0;
const WEEK_SECONDS: f64 = 604_800.0;
const DAY_SECONDS: f64 = 86_400.0;

pub struct GpsTimeConvert {
    in_time: String,
    out_time: String,
    wrap: bool,
    wrapped: bool,
    wrapped_tolerance: f64,
    first: bool,
    last_time: f64,
    num_seconds: i64,
}

impl GpsTimeConvert {
    pub fn from_options(options: &Options) -> Result<Self, StageError> {
        let conversion = options.get_str("conversion", "");
        let mut in_time = options.get_str("in_time", "");
        let mut out_time = options.get_str("out_time", "");
        if !conversion.is_empty() && in_time.is_empty() && out_time.is_empty() {
            let parts: Vec<_> = conversion
                .to_lowercase()
                .split('2')
                .map(str::to_string)
                .collect();
            if parts.len() < 2 {
                return Err(StageError(
                    "Invalid 'conversion'. Please use the format {in}2{out}, where {in} is the input time format and {out} is the output time format or use or 'in_time' and 'out_time' parameters.".to_string(),
                ));
            }
            in_time = parts[0].clone();
            out_time = parts[1].clone();
        } else if conversion.is_empty() && !in_time.is_empty() && !out_time.is_empty() {
            in_time = in_time.to_lowercase();
            out_time = out_time.to_lowercase();
        } else {
            return Err(StageError(
                "Use 'conversion' or 'in_time' and 'out_time'.".to_string(),
            ));
        }

        validate_time_type(&in_time)?;
        validate_time_type(&out_time)?;

        let start_date = options.get_str("start_date", "");
        let mut num_seconds = i64::MIN;
        if in_time == "gws" || in_time == "gds" {
            if start_date.is_empty() {
                return Err(StageError("'start_date' option is required.".to_string()));
            }
            let date = parse_date(&start_date)?;
            num_seconds = if in_time == "gws" {
                week_start_gps_seconds(date)
            } else {
                day_start_gps_seconds(date)
            };
            if out_time == "gst" {
                num_seconds -= GPS_TIME_OFFSET as i64;
            }
        }

        Ok(Self {
            in_time,
            out_time,
            wrap: options.get_bool("wrap", false),
            wrapped: options.get_bool("wrapped", false),
            wrapped_tolerance: options.get_f64("wrapped_tolerance", 1.0),
            first: true,
            last_time: f64::NEG_INFINITY,
            num_seconds,
        })
    }

    fn unwrap_seconds(&mut self, view: &mut PointView, idx: PointId, period: f64) {
        while view.get_f64(idx, &DimId::GpsTime) + self.wrapped_tolerance < self.last_time {
            let t = view.get_f64(idx, &DimId::GpsTime);
            view.set_f64(idx, &DimId::GpsTime, t + period);
        }
        self.last_time = view.get_f64(idx, &DimId::GpsTime);
    }

    fn wrap_seconds(view: &mut PointView, idx: PointId, period: f64) {
        while view.get_f64(idx, &DimId::GpsTime) >= period {
            let t = view.get_f64(idx, &DimId::GpsTime);
            view.set_f64(idx, &DimId::GpsTime, t - period);
        }
    }

    fn week_seconds_to_gps_time(&mut self, view: &mut PointView, idx: PointId) {
        if self.wrapped {
            self.unwrap_seconds(view, idx, WEEK_SECONDS);
        }
        let t = view.get_f64(idx, &DimId::GpsTime);
        view.set_f64(idx, &DimId::GpsTime, t + self.num_seconds as f64);
    }

    fn day_seconds_to_gps_time(&mut self, view: &mut PointView, idx: PointId) {
        if self.wrapped {
            self.unwrap_seconds(view, idx, DAY_SECONDS);
        }
        let t = view.get_f64(idx, &DimId::GpsTime);
        view.set_f64(idx, &DimId::GpsTime, t + self.num_seconds as f64);
    }

    fn gps_time_to_week_seconds(&self, view: &mut PointView, idx: PointId) {
        let t = view.get_f64(idx, &DimId::GpsTime);
        view.set_f64(idx, &DimId::GpsTime, t - self.num_seconds as f64);
        if self.wrap {
            Self::wrap_seconds(view, idx, WEEK_SECONDS);
        }
    }

    fn gps_time_to_day_seconds(&self, view: &mut PointView, idx: PointId) {
        let t = view.get_f64(idx, &DimId::GpsTime);
        view.set_f64(idx, &DimId::GpsTime, t - self.num_seconds as f64);
        if self.wrap {
            Self::wrap_seconds(view, idx, DAY_SECONDS);
        }
    }

    fn gps_time_to_gps_time(&self, view: &mut PointView, idx: PointId) {
        let offset = if self.in_time == "gt" && self.out_time == "gst" {
            -GPS_TIME_OFFSET
        } else {
            GPS_TIME_OFFSET
        };
        let t = view.get_f64(idx, &DimId::GpsTime);
        view.set_f64(idx, &DimId::GpsTime, t + offset);
    }
}

impl Filter for GpsTimeConvert {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.gpstimeconvert"
    }

    fn run(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut output = input.clone();
        for idx in 0..output.len() {
            self.process_one(&mut output, idx);
        }
        Ok(vec![output])
    }
}

impl Streamable for GpsTimeConvert {
    fn process_one(&mut self, view: &mut PointView, idx: PointId) -> bool {
        if self.in_time == "gws" {
            if self.out_time == "gst" || self.out_time == "gt" {
                self.week_seconds_to_gps_time(view, idx);
            } else if self.out_time == "gds" {
                self.week_seconds_to_gps_time(view, idx);
                self.gps_time_to_day_seconds(view, idx);
            }
        } else if self.in_time == "gds" {
            if self.out_time == "gst" || self.out_time == "gt" {
                self.day_seconds_to_gps_time(view, idx);
            }
            if self.out_time == "gws" {
                self.day_seconds_to_gps_time(view, idx);
                self.gps_time_to_week_seconds(view, idx);
            }
        } else if self.in_time == "gst" || self.in_time == "gt" {
            if self.first {
                let t_offset = if self.in_time == "gst" {
                    GPS_TIME_OFFSET as i64
                } else {
                    0
                };
                let t = view.get_f64(idx, &DimId::GpsTime) as i64 + t_offset;
                let first_date = gps_time_to_date(t);
                if self.out_time == "gds" {
                    self.num_seconds = day_start_gps_seconds(first_date) - t_offset;
                } else if self.out_time == "gws" {
                    self.num_seconds = week_start_gps_seconds(first_date) - t_offset;
                }
                self.first = false;
            }
            if self.out_time == "gws" {
                self.gps_time_to_week_seconds(view, idx);
            } else if self.out_time == "gds" {
                self.gps_time_to_day_seconds(view, idx);
            } else if (self.out_time == "gst" || self.out_time == "gt")
                && self.in_time != self.out_time
            {
                self.gps_time_to_gps_time(view, idx);
            }
        }
        true
    }

    fn reset(&mut self) {
        self.first = true;
        self.last_time = f64::NEG_INFINITY;
    }
}

#[derive(Clone, Copy)]
struct Date {
    year: i32,
    month: u32,
    day: u32,
}

fn validate_time_type(value: &str) -> Result<(), StageError> {
    match value {
        "gt" | "gst" | "gws" | "gds" => Ok(()),
        _ => Err(StageError("Invalid time type.".to_string())),
    }
}

fn parse_date(value: &str) -> Result<Date, StageError> {
    let parts: Vec<_> = value.split('-').collect();
    if parts.len() != 3 {
        return Err(StageError(
            "'start_date' must be in YYYY-MM-DD format.".to_string(),
        ));
    }
    let year = parts[0]
        .parse()
        .map_err(|_| StageError("'start_date' must be in YYYY-MM-DD format.".to_string()))?;
    let month = parts[1]
        .parse()
        .map_err(|_| StageError("'start_date' must be in YYYY-MM-DD format.".to_string()))?;
    let day = parts[2]
        .parse()
        .map_err(|_| StageError("'start_date' must be in YYYY-MM-DD format.".to_string()))?;
    Ok(Date { year, month, day })
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = year - (month <= 2) as i32;
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let month = month as i32;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day as i32 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    (era * 146097 + doe - 719468) as i64
}

fn weekday_sunday_zero(days_since_unix_epoch: i64) -> i64 {
    (days_since_unix_epoch + 4).rem_euclid(7)
}

fn gps_zero_days() -> i64 {
    days_from_civil(1980, 1, 6)
}

fn day_start_gps_seconds(date: Date) -> i64 {
    (days_from_civil(date.year, date.month, date.day) - gps_zero_days()) * DAY_SECONDS as i64
}

fn week_start_gps_seconds(date: Date) -> i64 {
    let days = days_from_civil(date.year, date.month, date.day);
    let week_start = days - weekday_sunday_zero(days);
    (week_start - gps_zero_days()) * DAY_SECONDS as i64
}

fn gps_time_to_date(seconds: i64) -> Date {
    civil_from_days(gps_zero_days() + seconds.div_euclid(DAY_SECONDS as i64))
}

fn civil_from_days(days: i64) -> Date {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    Date {
        year: (y + (month <= 2) as i64) as i32,
        month: month as u32,
        day: day as u32,
    }
}
