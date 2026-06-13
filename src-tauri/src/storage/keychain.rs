use anyhow::Result;
use keyring::Entry;

const SERVICE: &str = "loflum-ftp-client";

pub fn store_password(site_id: &str, password: &str) -> Result<()> {
    let entry = Entry::new(SERVICE, site_id)?;
    entry.set_password(password)?;
    Ok(())
}

pub fn get_password(site_id: &str) -> Result<String> {
    let entry = Entry::new(SERVICE, site_id)?;
    Ok(entry.get_password()?)
}

pub fn delete_password(site_id: &str) -> Result<()> {
    let entry = Entry::new(SERVICE, site_id)?;
    entry.delete_password()?;
    Ok(())
}
