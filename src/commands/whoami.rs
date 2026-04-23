use crate::client::{ClockifyClient, HttpTransport};
use crate::error::CfdError;
use crate::format::{format_json, format_text_fields, OutputFormat, OutputOptions, TextField};

pub fn execute<T: HttpTransport>(
    client: &ClockifyClient<T>,
    output: &OutputOptions,
) -> Result<(), CfdError> {
    let user = client.get_current_user()?;

    match output.format {
        OutputFormat::Json => {
            println!("{}", format_json(&user)?);
        }
        OutputFormat::Text => {
            println!(
                "{}",
                format_text_fields(
                    &[
                        TextField {
                            label: "id",
                            value: &user.id,
                            is_meta: true,
                        },
                        TextField {
                            label: "name",
                            value: &user.name,
                            is_meta: false,
                        },
                        TextField {
                            label: "email",
                            value: &user.email,
                            is_meta: false,
                        },
                    ],
                    output,
                )
            );
        }
    }

    Ok(())
}
