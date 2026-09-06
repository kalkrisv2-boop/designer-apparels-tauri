// Converts a rupee amount (with up to 2 decimal paise) into words using the
// Indian numbering system (Thousand / Lakh / Crore), matching the style used
// on the reference invoice: "Eighteen Thousand Five Hundred Sixty Nine Rupees Only".

const ONES: [&str; 20] = [
    "Zero", "One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight", "Nine", "Ten",
    "Eleven", "Twelve", "Thirteen", "Fourteen", "Fifteen", "Sixteen", "Seventeen", "Eighteen",
    "Nineteen",
];

const TENS: [&str; 10] = [
    "", "", "Twenty", "Thirty", "Forty", "Fifty", "Sixty", "Seventy", "Eighty", "Ninety",
];

fn two_digits(n: u64) -> String {
    if n == 0 {
        return String::new();
    }
    if n < 20 {
        return ONES[n as usize].to_string();
    }
    let tens = n / 10;
    let ones = n % 10;
    if ones == 0 {
        TENS[tens as usize].to_string()
    } else {
        format!("{} {}", TENS[tens as usize], ONES[ones as usize])
    }
}

fn three_digits(n: u64) -> String {
    if n == 0 {
        return String::new();
    }
    let hundreds = n / 100;
    let rest = n % 100;
    let mut parts = vec![];
    if hundreds > 0 {
        parts.push(format!("{} Hundred", ONES[hundreds as usize]));
    }
    if rest > 0 {
        parts.push(two_digits(rest));
    }
    parts.join(" ")
}

/// Converts a whole (non-negative) integer into Indian-style words.
fn number_to_words_indian(mut n: u64) -> String {
    if n == 0 {
        return "Zero".to_string();
    }

    let crore = n / 1_00_00_000;
    n %= 1_00_00_000;
    let lakh = n / 1_00_000;
    n %= 1_00_000;
    let thousand = n / 1_000;
    n %= 1_000;
    let hundred_rest = n;

    let mut parts = vec![];
    if crore > 0 {
        parts.push(format!("{} Crore", three_digits(crore)));
    }
    if lakh > 0 {
        parts.push(format!("{} Lakh", three_digits(lakh)));
    }
    if thousand > 0 {
        parts.push(format!("{} Thousand", three_digits(thousand)));
    }
    if hundred_rest > 0 {
        parts.push(three_digits(hundred_rest));
    }
    parts.join(" ")
}

/// Converts a rupee amount (f64) into a full words string, e.g.
/// `amount_to_words(18569.0)` -> "Rupees Eighteen Thousand Five Hundred Sixty Nine Only"
pub fn amount_to_words(amount: f64) -> String {
    let rounded = (amount * 100.0).round() / 100.0;
    let rupees = rounded.trunc() as u64;
    let paise = ((rounded - rupees as f64) * 100.0).round() as u64;

    let rupee_words = number_to_words_indian(rupees);

    if paise > 0 {
        let paise_words = number_to_words_indian(paise);
        format!("Rupees {} and {} Paise Only", rupee_words, paise_words)
    } else {
        format!("Rupees {} Only", rupee_words)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_sample_invoice_total() {
        // Reference invoice: Grand Total 18569.00 ->
        // "Eighteen Thousand Five Hundred Sixty Nine Rupees Only"
        assert_eq!(
            amount_to_words(18569.0),
            "Rupees Eighteen Thousand Five Hundred Sixty Nine Only"
        );
    }

    #[test]
    fn handles_lakhs_and_paise() {
        assert_eq!(
            amount_to_words(105230.50),
            "Rupees One Lakh Five Thousand Two Hundred Thirty and Fifty Paise Only"
        );
    }
}
