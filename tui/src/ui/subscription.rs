//! 订阅信息格式化：统一输出 Provider 余额和 UTC 到期日期。
//! 修改时间：2026-07-28 18:15:12 +08:00

const BYTES_PER_GB: f64 = 1_000_000_000.0;

pub fn label(remaining: Option<u64>, expire: Option<i64>) -> String {
    let remaining = remaining
        .map(|bytes| format!("{:.1} GB", bytes as f64 / BYTES_PER_GB))
        .unwrap_or_else(|| "-".into());
    let expire = expire.and_then(unix_date).unwrap_or_else(|| "-".into());
    format!("remain {remaining} expire {expire}")
}

fn unix_date(timestamp: i64) -> Option<String> {
    if timestamp <= 0 {
        return None;
    }

    // Unix 时间按 UTC 天数换算为公历日期，避免引入完整日期时间依赖。
    let days = timestamp.div_euclid(86_400);
    let shifted = days.checked_add(719_468)?;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    }
    .div_euclid(146_097);
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    Some(format!("{year:04}-{month:02}-{day:02}"))
}

#[cfg(test)]
mod tests {
    use super::label;

    #[test]
    fn formats_remaining_balance_and_expiration_date() {
        assert_eq!(
            label(Some(76_600_000_000), Some(1_788_134_400)),
            "remain 76.6 GB expire 2026-08-31"
        );
    }

    #[test]
    fn formats_missing_subscription_info() {
        assert_eq!(label(None, None), "remain - expire -");
    }
}
