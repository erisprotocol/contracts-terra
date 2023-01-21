use crate::error::ContractError;
use crate::error::ContractError::MarketingInfoValidationError;

use cosmwasm_std::StdError;
use cw20::{EmbeddedLogo, Logo};

const SAFE_TEXT_CHARS: &str = "!&?#()*+'-.,/\"";
const SAFE_LINK_CHARS: &str = "-_:/?#@!$&()*+,;=.~[]'%";
const LOGO_SIZE_CAP: usize = 5 * 1024;

fn validate_text(text: &str, name: &str) -> Result<(), ContractError> {
    if text.chars().any(|c| {
        !c.is_ascii_alphanumeric() && !c.is_ascii_whitespace() && !SAFE_TEXT_CHARS.contains(c)
    }) {
        Err(MarketingInfoValidationError(
            format!("{} contains invalid characters: {}", name, text,),
        ))
    } else {
        Ok(())
    }
}

pub fn validate_whitelist_links(links: &[String]) -> Result<(), ContractError> {
    links.iter().try_for_each(|link| {
        if !link.ends_with('/') {
            return Err(MarketingInfoValidationError(format!(
                "Whitelist link should end with '/': {}",
                link,
            )));
        }
        validate_link(link)
    })
}

pub fn validate_link(link: &str) -> Result<(), ContractError> {
    if link.chars().any(|c| !c.is_ascii_alphanumeric() && !SAFE_LINK_CHARS.contains(c)) {
        Err(StdError::generic_err(format!("Link contains invalid characters: {}", link)).into())
    } else {
        Ok(())
    }
}

fn check_link(link: &str, whitelisted_links: &[String]) -> Result<(), ContractError> {
    if validate_link(link).is_err() {
        Err(MarketingInfoValidationError(format!("Logo link is invalid: {}", link)))
    } else if !whitelisted_links.iter().any(|wl| link.starts_with(wl)) {
        Err(MarketingInfoValidationError(format!("Logo link is not whitelisted: {}", link)))
    } else {
        Ok(())
    }
}

pub(crate) fn validate_marketing_info(
    project: Option<&String>,
    description: Option<&String>,
    logo: Option<&Logo>,
    whitelisted_links: &[String],
) -> Result<(), ContractError> {
    if let Some(description) = description {
        validate_text(description, "description")?;
    }
    if let Some(project) = project {
        validate_text(project, "project")?;
    }
    if let Some(Logo::Url(url)) = logo {
        check_link(url, whitelisted_links)?;
    }
    if let Some(logo) = logo {
        verify_logo(logo)?;
    }

    Ok(())
}

/// Checks if data starts with XML preamble
fn verify_xml_preamble(data: &[u8]) -> Result<(), ContractError> {
    // The easiest way to perform this check would be just match on regex, however regex
    // compilation is heavy and probably not worth it.

    let preamble =
        data.split_inclusive(|c| *c == b'>').next().ok_or(ContractError::InvalidXmlPreamble {})?;

    const PREFIX: &[u8] = b"<?xml ";
    const POSTFIX: &[u8] = b"?>";

    if !(preamble.starts_with(PREFIX) && preamble.ends_with(POSTFIX)) {
        Err(ContractError::InvalidXmlPreamble {})
    } else {
        Ok(())
    }

    // Additionally attributes format could be validated as they are well defined, as well as
    // comments presence inside of preable, but it is probably not worth it.
}

/// Validates XML logo
fn verify_xml_logo(logo: &[u8]) -> Result<(), ContractError> {
    verify_xml_preamble(logo)?;

    if logo.len() > LOGO_SIZE_CAP {
        Err(ContractError::LogoTooBig {})
    } else {
        Ok(())
    }
}

/// Validates png logo
fn verify_png_logo(logo: &[u8]) -> Result<(), ContractError> {
    // PNG header format:
    // 0x89 - magic byte, out of ASCII table to fail on 7-bit systems
    // "PNG" ascii representation
    // [0x0d, 0x0a] - dos style line ending
    // 0x1a - dos control character, stop displaying rest of the file
    // 0x0a - unix style line ending
    const HEADER: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    if logo.len() > LOGO_SIZE_CAP {
        Err(ContractError::LogoTooBig {})
    } else if !logo.starts_with(&HEADER) {
        Err(ContractError::InvalidPngHeader {})
    } else {
        Ok(())
    }
}

/// Checks if passed logo is correct, and if not, returns an error
fn verify_logo(logo: &Logo) -> Result<(), ContractError> {
    match logo {
        Logo::Embedded(EmbeddedLogo::Svg(logo)) => verify_xml_logo(logo),
        Logo::Embedded(EmbeddedLogo::Png(logo)) => verify_png_logo(logo),
        Logo::Url(_) => Ok(()), // Any reasonable url validation would be regex based, probably not worth it
    }
}
