use dialoguer::{Select, Input};
use prettytable::{Table, row};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use rust_decimal::prelude::*;
use serde::Deserialize;
use std::fs;

#[derive(Debug, Clone)]
enum LoanType {
    Home,
    Car,
    Personal,
}

impl LoanType {
    fn get_default_amount(&self) -> f64 {
        match self {
            LoanType::Home => 300_000.0,
            LoanType::Car => 25_000.0,
            LoanType::Personal => 10_000.0,
        }
    }

    fn get_max_amount(&self) -> f64 {
        match self {
            LoanType::Home => 10_000_000.0,
            LoanType::Car => 150_000.0,
            LoanType::Personal => 100_000.0,
        }
    }

    fn get_description(&self) -> &str {
        match self {
            LoanType::Home => "Home loans typically range from $100,000 to $10,000,000",
            LoanType::Car => "Car loans typically range from $5,000 to $150,000",
            LoanType::Personal => "Personal loans typically range from $1,000 to $100,000",
        }
    }

    fn get_default_term(&self) -> u32 {
        match self {
            LoanType::Home => 30,
            LoanType::Car => 5,
            LoanType::Personal => 3,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct RateRange {
    min: f64,
    max: f64,
}

impl RateRange {
    fn to_decimal_tuple(&self) -> (Decimal, Decimal) {
        (
            Decimal::from_f64(self.min).unwrap(),
            Decimal::from_f64(self.max).unwrap(),
        )
    }
}

#[derive(Debug, Clone, Deserialize)]
struct BankConfig {
    name: String,
    home_loan_range: RateRange,
    car_loan_range: RateRange,
    personal_loan_range: RateRange,
    min_credit_score: u16,
}

#[derive(Debug, Clone)]
struct Bank {
    name: String,
    home_loan_range: (Decimal, Decimal),
    car_loan_range: (Decimal, Decimal),
    personal_loan_range: (Decimal, Decimal),
    min_credit_score: u16,
}

impl From<BankConfig> for Bank {
    fn from(config: BankConfig) -> Self {
        Bank {
            name: config.name,
            home_loan_range: config.home_loan_range.to_decimal_tuple(),
            car_loan_range: config.car_loan_range.to_decimal_tuple(),
            personal_loan_range: config.personal_loan_range.to_decimal_tuple(),
            min_credit_score: config.min_credit_score,
        }
    }
}

#[derive(Debug, Deserialize)]
struct BanksConfig {
    banks: Vec<BankConfig>,
}

impl Bank {
    fn get_rate_range(&self, loan_type: &LoanType) -> (Decimal, Decimal) {
        match loan_type {
            LoanType::Home => self.home_loan_range,
            LoanType::Car => self.car_loan_range,
            LoanType::Personal => self.personal_loan_range,
        }
    }
}

struct LoanCalculator {
    banks: Vec<Bank>,
}

impl LoanCalculator {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let config_str = fs::read_to_string("banks.yaml")?;
        let config: BanksConfig = serde_yaml::from_str(&config_str)?;
        let banks = config.banks.into_iter().map(Bank::from).collect();
        Ok(Self { banks })
    }

    fn decimal_pow(&self, base: Decimal, exp: u32) -> Decimal {
        let mut result = dec!(1);
        let mut base = base;
        let mut exp = exp;

        while exp > 0 {
            if exp & 1 == 1 {
                result *= base;
            }
            base *= base;
            exp >>= 1;
        }

        result
    }

    fn calculate_monthly_payment(&self, principal: Decimal, annual_rate: Decimal, years: u32) -> Decimal {
        let monthly_rate = annual_rate / dec!(100) / dec!(12);
        let num_payments = years * 12;
        
        let base = dec!(1) + monthly_rate;
        let base_raised = self.decimal_pow(base, num_payments);
        
        if base_raised == dec!(1) {
            return principal / Decimal::from(num_payments);
        }
        
        let numerator = monthly_rate * base_raised;
        let denominator = base_raised - dec!(1);
        
        principal * (numerator / denominator)
    }

    fn adjust_rate_for_credit(&self, base_rate: Decimal, credit_score: u16) -> Decimal {
        match credit_score {
            score if score >= 800 => base_rate - dec!(0.5),
            score if score >= 750 => base_rate - dec!(0.25),
            score if score >= 700 => base_rate,
            score if score >= 650 => base_rate + dec!(0.5),
            score if score >= 600 => base_rate + dec!(1.0),
            _ => base_rate + dec!(2.0),
        }
    }

    fn get_min_credit_score(&self) -> u16 {
        self.banks.iter().map(|bank| bank.min_credit_score).min().unwrap_or(300)
    }
}

fn format_money(amount: Decimal) -> String {
    let mut str_amount = format!("{:.2}", amount);
    let decimal_pos = str_amount.find('.').unwrap_or(str_amount.len());
    let mut pos = decimal_pos;
    while pos > 3 {
        pos -= 3;
        str_amount.insert(pos, ',');
    }
    format!("${}", str_amount)
}

fn get_valid_credit_score() -> Result<u16, Box<dyn std::error::Error>> {
    loop {
        let score: u16 = Input::new()
            .with_prompt("Enter your credit score (300-850)")
            .validate_with(|input: &u16| {
                if *input >= 300 && *input <= 850 {
                    Ok(())
                } else {
                    Err("Credit score must be between 300 and 850")
                }
            })
            .interact_text()?;
        return Ok(score);
    }
}

fn get_valid_loan_amount(loan_type: &LoanType) -> Result<Decimal, Box<dyn std::error::Error>> {
    println!("\n{}", loan_type.get_description());
    loop {
        let amount: f64 = Input::new()
            .with_prompt("Enter loan amount ($)")
            .with_initial_text(&format!("{}", loan_type.get_default_amount()))
            .validate_with(move |input: &f64| -> Result<(), &str> {
                if *input <= 0.0 {
                    Err("Loan amount must be greater than 0")
                } else if *input > loan_type.get_max_amount() {
                    Err("Loan amount exceeds maximum allowed")
                } else {
                    Ok(())
                }
            })
            .interact_text()?;
        return Ok(Decimal::from_f64(amount).unwrap());
    }
}

fn get_valid_loan_term(loan_type: &LoanType) -> Result<u32, Box<dyn std::error::Error>> {
    loop {
        let term: u32 = Input::new()
            .with_prompt("Enter loan term (1-30 years)")
            .with_initial_text(&format!("{}", loan_type.get_default_term()))
            .validate_with(|input: &u32| {
                if *input >= 1 && *input <= 30 {
                    Ok(())
                } else {
                    Err("Loan term must be between 1 and 30 years")
                }
            })
            .interact_text()?;
        return Ok(term);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let calculator = LoanCalculator::new()?;
    
    // Select loan type
    let loan_types = vec!["Home Loan", "Car Loan", "Personal Loan"];
    let loan_type_selection = Select::new()
        .with_prompt("Select loan type")
        .items(&loan_types)
        .default(0)
        .interact()?;
    
    let loan_type = match loan_type_selection {
        0 => LoanType::Home,
        1 => LoanType::Car,
        2 => LoanType::Personal,
        _ => unreachable!(),
    };

    // Get loan details with validation
    let loan_amount = get_valid_loan_amount(&loan_type)?;
    let loan_term = get_valid_loan_term(&loan_type)?;
    let credit_score = get_valid_credit_score()?;

    // Create results table
    let mut table = Table::new();
    table.add_row(row![
        "Bank",
        "Interest Rate",
        "Monthly Payment",
        "Total Interest",
        "Total Payment"
    ]);

    let mut has_qualifying_banks = false;

    for bank in &calculator.banks {
        let (min_rate, max_rate) = bank.get_rate_range(&loan_type);
        
        // Skip if credit score is too low
        if credit_score < bank.min_credit_score {
            continue;
        }

        has_qualifying_banks = true;

        // Calculate adjusted rate based on credit score
        let base_rate = (min_rate + max_rate) / dec!(2);
        let adjusted_rate = calculator.adjust_rate_for_credit(base_rate, credit_score);
        
        let monthly_payment = calculator.calculate_monthly_payment(
            loan_amount,
            adjusted_rate,
            loan_term,
        );
        
        let total_payment = monthly_payment * Decimal::from(loan_term * 12);
        let total_interest = total_payment - loan_amount;

        table.add_row(row![
            bank.name,
            format!("{:.2}%", adjusted_rate),
            format_money(monthly_payment),
            format_money(total_interest),
            format_money(total_payment)
        ]);
    }

    if !has_qualifying_banks {
        println!("\nNo banks available for credit score {}.", credit_score);
        println!("Minimum required credit score is {}.", calculator.get_min_credit_score());
        println!("Consider using a custom interest rate to estimate payments.");
    }

    // Option for custom rate
    println!("\nWould you like to calculate with a custom interest rate?");
    let custom_rate_options = vec!["Yes", "No"];
    let custom_rate_selection = Select::new()
        .items(&custom_rate_options)
        .default(1)
        .interact()?;

    if custom_rate_selection == 0 {
        let custom_rate: f64 = Input::new()
            .with_prompt("Enter custom interest rate (%)")
            .validate_with(|input: &f64| {
                if *input > 0.0 && *input < 100.0 {
                    Ok(())
                } else {
                    Err("Interest rate must be between 0 and 100")
                }
            })
            .interact_text()?;
        let custom_rate = Decimal::from_f64(custom_rate).unwrap();
        
        let monthly_payment = calculator.calculate_monthly_payment(
            loan_amount,
            custom_rate,
            loan_term,
        );
        
        let total_payment = monthly_payment * Decimal::from(loan_term * 12);
        let total_interest = total_payment - loan_amount;

        table.add_row(row![
            "Custom Rate",
            format!("{:.2}%", custom_rate),
            format_money(monthly_payment),
            format_money(total_interest),
            format_money(total_payment)
        ]);
    }

    // Print loan details
    println!("\nLoan Details:");
    println!("Amount: {}", format_money(loan_amount));
    println!("Term: {} years", loan_term);
    println!("Credit Score: {}", credit_score);
    println!("\nComparison of Options:");
    table.printstd();

    Ok(())
}
