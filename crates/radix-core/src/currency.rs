//! Currency registry and rate providers (spec: doc 04 §3.2, doc 08).
//!
//! Rates are fetched online; a currency without a rate cannot be used
//! (conversion → UNKNOWN_RATE). Provider order: IMF, UN, ECB, Bank of
//! Canada — first to set a currency wins; later providers fill only
//! currencies still missing rates.

use crate::error::{messages, Error, ErrorCode};
use crate::number::Number;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CurrencyDef {
    pub code: &'static str,
    pub name: &'static str,
    pub symbol: &'static str,
}

/// Built-in currency list (doc 04 §3.2: 67 currencies; the spec's exact list
/// was not part of the handover, so the standard ISO set below is used —
/// provider files can add more, decision D-recorded).
pub const CURRENCIES: &[CurrencyDef] = &[
    CurrencyDef {
        code: "AED",
        name: "United Arab Emirates Dirham",
        symbol: "د.إ",
    },
    CurrencyDef {
        code: "AUD",
        name: "Australian Dollar",
        symbol: "A$",
    },
    CurrencyDef {
        code: "BGN",
        name: "Bulgarian Lev",
        symbol: "лв",
    },
    CurrencyDef {
        code: "BRL",
        name: "Brazilian Real",
        symbol: "R$",
    },
    CurrencyDef {
        code: "CAD",
        name: "Canadian Dollar",
        symbol: "C$",
    },
    CurrencyDef {
        code: "CHF",
        name: "Swiss Franc",
        symbol: "CHF",
    },
    CurrencyDef {
        code: "CNY",
        name: "Chinese Yuan",
        symbol: "¥",
    },
    CurrencyDef {
        code: "CZK",
        name: "Czech Koruna",
        symbol: "Kč",
    },
    CurrencyDef {
        code: "DKK",
        name: "Danish Krone",
        symbol: "kr",
    },
    CurrencyDef {
        code: "EUR",
        name: "Euro",
        symbol: "€",
    },
    CurrencyDef {
        code: "GBP",
        name: "British Pound Sterling",
        symbol: "£",
    },
    CurrencyDef {
        code: "HKD",
        name: "Hong Kong Dollar",
        symbol: "HK$",
    },
    CurrencyDef {
        code: "HUF",
        name: "Hungarian Forint",
        symbol: "Ft",
    },
    CurrencyDef {
        code: "IDR",
        name: "Indonesian Rupiah",
        symbol: "Rp",
    },
    CurrencyDef {
        code: "ILS",
        name: "Israeli New Shekel",
        symbol: "₪",
    },
    CurrencyDef {
        code: "INR",
        name: "Indian Rupee",
        symbol: "₹",
    },
    CurrencyDef {
        code: "ISK",
        name: "Icelandic Króna",
        symbol: "kr",
    },
    CurrencyDef {
        code: "JPY",
        name: "Japanese Yen",
        symbol: "¥",
    },
    CurrencyDef {
        code: "KRW",
        name: "South Korean Won",
        symbol: "₩",
    },
    CurrencyDef {
        code: "MXN",
        name: "Mexican Peso",
        symbol: "$",
    },
    CurrencyDef {
        code: "MYR",
        name: "Malaysian Ringgit",
        symbol: "RM",
    },
    CurrencyDef {
        code: "NOK",
        name: "Norwegian Krone",
        symbol: "kr",
    },
    CurrencyDef {
        code: "NZD",
        name: "New Zealand Dollar",
        symbol: "NZ$",
    },
    CurrencyDef {
        code: "PHP",
        name: "Philippine Peso",
        symbol: "₱",
    },
    CurrencyDef {
        code: "PLN",
        name: "Polish Zloty",
        symbol: "zł",
    },
    CurrencyDef {
        code: "RON",
        name: "Romanian Leu",
        symbol: "lei",
    },
    CurrencyDef {
        code: "SEK",
        name: "Swedish Krona",
        symbol: "kr",
    },
    CurrencyDef {
        code: "SGD",
        name: "Singapore Dollar",
        symbol: "S$",
    },
    CurrencyDef {
        code: "THB",
        name: "Thai Baht",
        symbol: "฿",
    },
    CurrencyDef {
        code: "TRY",
        name: "Turkish Lira",
        symbol: "₺",
    },
    CurrencyDef {
        code: "USD",
        name: "US Dollar",
        symbol: "$",
    },
    CurrencyDef {
        code: "ZAR",
        name: "South African Rand",
        symbol: "R",
    },
    CurrencyDef {
        code: "AFN",
        name: "Afghan Afghani",
        symbol: "؋",
    },
    CurrencyDef {
        code: "ALL",
        name: "Albanian Lek",
        symbol: "L",
    },
    CurrencyDef {
        code: "AMD",
        name: "Armenian Dram",
        symbol: "֏",
    },
    CurrencyDef {
        code: "ARS",
        name: "Argentine Peso",
        symbol: "$",
    },
    CurrencyDef {
        code: "AZN",
        name: "Azerbaijani Manat",
        symbol: "₼",
    },
    CurrencyDef {
        code: "BDT",
        name: "Bangladeshi Taka",
        symbol: "৳",
    },
    CurrencyDef {
        code: "BHD",
        name: "Bahraini Dinar",
        symbol: ".د.ب",
    },
    CurrencyDef {
        code: "BND",
        name: "Brunei Dollar",
        symbol: "B$",
    },
    CurrencyDef {
        code: "BOB",
        name: "Bolivian Boliviano",
        symbol: "Bs.",
    },
    CurrencyDef {
        code: "BWP",
        name: "Botswana Pula",
        symbol: "P",
    },
    CurrencyDef {
        code: "BYN",
        name: "Belarusian Ruble",
        symbol: "Br",
    },
    CurrencyDef {
        code: "CLP",
        name: "Chilean Peso",
        symbol: "$",
    },
    CurrencyDef {
        code: "COP",
        name: "Colombian Peso",
        symbol: "$",
    },
    CurrencyDef {
        code: "CRC",
        name: "Costa Rican Colón",
        symbol: "₡",
    },
    CurrencyDef {
        code: "DZD",
        name: "Algerian Dinar",
        symbol: "د.ج",
    },
    CurrencyDef {
        code: "EGP",
        name: "Egyptian Pound",
        symbol: "E£",
    },
    CurrencyDef {
        code: "ETB",
        name: "Ethiopian Birr",
        symbol: "Br",
    },
    CurrencyDef {
        code: "GEL",
        name: "Georgian Lari",
        symbol: "₾",
    },
    CurrencyDef {
        code: "GHS",
        name: "Ghanaian Cedi",
        symbol: "₵",
    },
    CurrencyDef {
        code: "HRK",
        name: "Croatian Kuna",
        symbol: "kn",
    },
    CurrencyDef {
        code: "IQD",
        name: "Iraqi Dinar",
        symbol: "ع.د",
    },
    CurrencyDef {
        code: "JOD",
        name: "Jordanian Dinar",
        symbol: "د.ا",
    },
    CurrencyDef {
        code: "KES",
        name: "Kenyan Shilling",
        symbol: "KSh",
    },
    CurrencyDef {
        code: "KGS",
        name: "Kyrgystani Som",
        symbol: "с",
    },
    CurrencyDef {
        code: "KHR",
        name: "Cambodian Riel",
        symbol: "៛",
    },
    CurrencyDef {
        code: "KWD",
        name: "Kuwaiti Dinar",
        symbol: "د.ك",
    },
    CurrencyDef {
        code: "KZT",
        name: "Kazakhstani Tenge",
        symbol: "₸",
    },
    CurrencyDef {
        code: "LBP",
        name: "Lebanese Pound",
        symbol: "ل.ل",
    },
    CurrencyDef {
        code: "LKR",
        name: "Sri Lankan Rupee",
        symbol: "₨",
    },
    CurrencyDef {
        code: "MAD",
        name: "Moroccan Dirham",
        symbol: "د.م.",
    },
    CurrencyDef {
        code: "MUR",
        name: "Mauritian Rupee",
        symbol: "₨",
    },
    CurrencyDef {
        code: "NGN",
        name: "Nigerian Naira",
        symbol: "₦",
    },
    CurrencyDef {
        code: "OMR",
        name: "Omani Rial",
        symbol: "ر.ع.",
    },
    CurrencyDef {
        code: "PEN",
        name: "Peruvian Sol",
        symbol: "S/",
    },
    CurrencyDef {
        code: "QAR",
        name: "Qatari Riyal",
        symbol: "ر.ق",
    },
    CurrencyDef {
        code: "UAH",
        name: "Ukrainian Hryvnia",
        symbol: "₴",
    },
    CurrencyDef {
        code: "UZS",
        name: "Uzbekistani Som",
        symbol: "лв",
    },
    CurrencyDef {
        code: "VND",
        name: "Vietnamese Dong",
        symbol: "₫",
    },
    CurrencyDef {
        code: "XAF",
        name: "Central African CFA Franc",
        symbol: "Fr",
    },
    CurrencyDef {
        code: "XOF",
        name: "West African CFA Franc",
        symbol: "Fr",
    },
];

#[derive(Debug, Clone)]
pub struct Rate {
    /// Value of one unit of the currency (in a common base).
    pub value: f64,
    pub date: String,
    pub provider: &'static str,
}

/// Rate table: first provider to set a currency wins (doc 04 §3.2).
#[derive(Debug, Clone, Default)]
pub struct CurrencyTable {
    pub rates: HashMap<String, Rate>,
}

impl CurrencyTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn has_rate(&self, code: &str) -> bool {
        self.rates.contains_key(code)
    }

    /// True when the code names a known currency, regardless of rate
    /// availability (the lexer classifies the word; a missing rate then
    /// surfaces as UNKNOWN_RATE at conversion time — UNIT-T023).
    pub fn is_known(&self, code: &str) -> bool {
        CURRENCIES.iter().any(|d| d.code == code)
    }

    pub fn set_rate(&mut self, code: &str, value: f64, date: &str, provider: &'static str) {
        self.rates.entry(code.to_string()).or_insert(Rate {
            value,
            date: date.to_string(),
            provider,
        });
    }

    /// `x from→to = x / value(from) × value(to)` (doc 04 §3.2).
    pub fn convert(&self, x: &Number, from: &str, to: &str, prec: u32) -> Result<Number, Error> {
        let fv = self.rates.get(from).ok_or_else(|| {
            Error::with_message(ErrorCode::UnknownRate, messages::unknown_rate(from))
        })?;
        let tv = self.rates.get(to).ok_or_else(|| {
            Error::with_message(ErrorCode::UnknownRate, messages::unknown_rate(to))
        })?;
        let f = Number::from_f64(fv.value, prec);
        let t = Number::from_f64(tv.value, prec);
        Ok(x.div(&f)?.mul(&t))
    }
}

// ----------------------------------------------------------------------
// Provider file parsers (formats per doc 04 §3.2; format drift is a
// provider concern — OQ-06 — so parsers are lenient and fixture-tested).
// ----------------------------------------------------------------------

/// ECB `eurofxref-daily.xml`: value = EUR-value × raw rate (EUR = 1);
/// fixed rates for RSD, XAF, XOF when absent (decision D10).
pub fn parse_ecb(xml: &str, table: &mut CurrencyTable) {
    let date = xml_attribute(xml, "Cube", "time").unwrap_or_default();
    let mut entries: Vec<(String, f64)> = Vec::new();
    for cap in xml.split("<Cube ").skip(1) {
        if let (Some(cur), Some(rate)) = (xml_attr(cap, "currency"), xml_attr(cap, "rate")) {
            if let Ok(v) = rate.trim().parse::<f64>() {
                entries.push((cur.trim().to_string(), v));
            }
        }
    }
    table.set_rate("EUR", 1.0, &date, "ECB");
    for (cur, rate) in entries {
        table.set_rate(&cur, rate, &date, "ECB");
    }
    // Fixed rates: value = EUR-value ÷ raw (doc 04 §3.2).
    let eur = table.rates.get("EUR").map(|r| r.value).unwrap_or(1.0);
    for (code, raw) in [("RSD", 0.0085f64), ("XAF", 0.00152449), ("XOF", 0.00152449)] {
        if !table.has_rate(code) {
            table.set_rate(code, eur / raw, &date, "ECB");
        }
    }
}

/// UN Treasury `un-daily.xls`: CRLF TSV, fields [2]=symbol, [3]=date,
/// [4]=rate; value = USD-value × raw (raw is the USD value of the currency).
pub fn parse_unt(text: &str, table: &mut CurrencyTable) {
    for line in text.lines() {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 5 {
            continue;
        }
        let symbol = fields[2].trim().to_uppercase();
        let date = fields[3].trim().to_string();
        let raw: f64 = match fields[4].trim().parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        table.set_rate(&symbol, raw, &date, "UNT");
    }
}

/// IMF `rms_five.xls`: rates are reciprocals of the raw file values; the
/// file maps currency *names* to rates (36 name-mapped currencies).
pub fn parse_imf(text: &str, table: &mut CurrencyTable) {
    for line in text.lines() {
        let fields: Vec<&str> = line.split('\t').map(|s| s.trim()).collect();
        if fields.len() < 2 {
            continue;
        }
        let name = fields[0];
        let raw: f64 = match fields[1].parse() {
            Ok(v) if v != 0.0 => v,
            _ => continue,
        };
        if let Some(code) = imf_name_to_code(name) {
            table.set_rate(code, 1.0 / raw, "", "IMF");
        }
    }
}

/// Bank of Canada `fxtwdcad.xml`: TWD value = CAD-value / raw.
pub fn parse_boc(xml: &str, table: &mut CurrencyTable) {
    // Observations carry USD/CAD and TWD/CAD series values.
    let mut usdcad: Option<f64> = None;
    let mut twdcad: Option<f64> = None;
    let date = xml_attribute(xml, "observation", "id").unwrap_or_default();
    for seg in xml.split("<v>").skip(1) {
        let val = seg.split('<').next().unwrap_or("");
        // Order of series in the document: FXUSDCAD then FXTWDCAD.
        if usdcad.is_none() {
            usdcad = val.trim().parse::<f64>().ok();
        } else if twdcad.is_none() {
            twdcad = val.trim().parse::<f64>().ok();
        }
    }
    if let (Some(usd), Some(twd)) = (usdcad, twdcad) {
        let cad_value = 1.0 / usd; // CAD in USD
        table.set_rate("CAD", cad_value, &date, "BOC");
        table.set_rate("TWD", cad_value / twd, &date, "BOC");
        table.set_rate("USD", 1.0, &date, "BOC");
    }
}

fn xml_attr(seg: &str, name: &str) -> Option<String> {
    let needle = format!("{name}=\"");
    let start = seg.find(&needle)? + needle.len();
    let rest = &seg[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn xml_attribute(xml: &str, tag: &str, attr: &str) -> Option<String> {
    let needle = format!("<{tag} ");
    let start = xml.find(&needle)?;
    xml_attr(&xml[start..], attr)
}

/// IMF currency-name → code map (36 name-mapped currencies, doc 04 §3.2).
fn imf_name_to_code(name: &str) -> Option<&'static str> {
    let n = name.trim();
    Some(match n {
        "Australian Dollar" => "AUD",
        "Brazilian Real" => "BRL",
        "Canadian Dollar" => "CAD",
        "Chinese Yuan" => "CNY",
        "Czech Koruna" => "CZK",
        "Danish Krone" => "DKK",
        "Euro" => "EUR",
        "Hong Kong Dollar" => "HKD",
        "Hungarian Forint" => "HUF",
        "Indian Rupee" => "INR",
        "Israeli New Shekel" => "ILS",
        "Japanese Yen" => "JPY",
        "Korean Won" => "KRW",
        "Malaysian Ringgit" => "MYR",
        "Mexican Peso" => "MXN",
        "New Zealand Dollar" => "NZD",
        "Norwegian Krone" => "NOK",
        "Polish Zloty" => "PLN",
        "Russian Ruble" => "RUB",
        "Singapore Dollar" => "SGD",
        "South African Rand" => "ZAR",
        "Swedish Krona" => "SEK",
        "Swiss Franc" => "CHF",
        "Thai Baht" => "THB",
        "U.S. Dollar" => "USD",
        "Pound Sterling" => "GBP",
        "Turkish Lira" => "TRY",
        "New Taiwan Dollar" => "TWD",
        "Philippine Peso" => "PHP",
        "Indonesian Rupiah" => "IDR",
        "Chilean Peso" => "CLP",
        "Colombian Peso" => "COP",
        "Peruvian Sol" => "PEN",
        "Icelandic Krona" => "ISK",
        "Romanian Leu" => "RON",
        "Bulgarian Lev" => "BGN",
        _ => return None,
    })
}
