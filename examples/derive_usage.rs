use llsd_rs::Llsd;
use std::collections::HashMap;

// Example showing manual implementation of derive patterns
// This demonstrates how the derive macros would work when implemented

#[derive(Debug, Clone, PartialEq)]
struct Person {
    pub first_name: String,
    pub last_name: String,
    pub age: u32,
    pub email: Option<String>,
}

// Manual implementation following derive patterns
impl TryFrom<&Llsd> for Person {
    type Error = anyhow::Error;

    fn try_from(llsd: &Llsd) -> anyhow::Result<Self> {
        if let Some(map) = llsd.as_map() {
            Ok(Self {
                first_name: map.get("first_name")
                    .ok_or_else(|| anyhow::Error::msg("Missing required field: first_name"))?
                    .try_into()?,
                last_name: map.get("last_name")
                    .ok_or_else(|| anyhow::Error::msg("Missing required field: last_name"))?
                    .try_into()?,
                age: *map.get("age")
                    .ok_or_else(|| anyhow::Error::msg("Missing required field: age"))?
                    .as_integer()
                    .ok_or_else(|| anyhow::Error::msg("Expected integer for age"))? as u32,
                email: map.get("email")
                    .map(String::try_from)
                    .transpose()?
            })
        } else {
            Err(anyhow::Error::msg("Expected LLSD Map"))
        }
    }
}

impl From<&Person> for Llsd {
    fn from(person: &Person) -> Self {
        let mut map = HashMap::new();
        map.insert("first_name".to_string(), Llsd::from(person.first_name.as_str()));
        map.insert("last_name".to_string(), Llsd::from(person.last_name.as_str()));
        map.insert("age".to_string(), Llsd::from(person.age as i32));
        if let Some(ref email) = person.email {
            map.insert("email".to_string(), Llsd::from(email.as_str()));
        }
        Llsd::Map(map)
    }
}

impl From<Person> for Llsd {
    fn from(person: Person) -> Self {
        Llsd::from(&person)
    }
}

// Example with camelCase field names (using rename_all = "camelCase" pattern)
#[derive(Debug, Clone, PartialEq)]
struct UserProfile {
    pub user_id: u64,
    pub display_name: String,
    pub is_active: bool,
}

impl TryFrom<&Llsd> for UserProfile {
    type Error = anyhow::Error;

    fn try_from(llsd: &Llsd) -> anyhow::Result<Self> {
        if let Some(map) = llsd.as_map() {
            Ok(Self {
                // Using camelCase field names as specified by rename_all = "camelCase"
                user_id: *map.get("userId")
                    .ok_or_else(|| anyhow::Error::msg("Missing required field: userId"))?
                    .as_integer()
                    .ok_or_else(|| anyhow::Error::msg("Expected integer for userId"))? as u64,
                display_name: map.get("displayName")
                    .ok_or_else(|| anyhow::Error::msg("Missing required field: displayName"))?
                    .try_into()?,
                is_active: *map.get("isActive")
                    .ok_or_else(|| anyhow::Error::msg("Missing required field: isActive"))?
                    .as_boolean()
                    .ok_or_else(|| anyhow::Error::msg("Expected boolean for isActive"))?,
            })
        } else {
            Err(anyhow::Error::msg("Expected LLSD Map"))
        }
    }
}

impl From<&UserProfile> for Llsd {
    fn from(profile: &UserProfile) -> Self {
        let mut map = HashMap::new();
        // Using camelCase field names
        map.insert("userId".to_string(), Llsd::from(profile.user_id as i32));
        map.insert("displayName".to_string(), Llsd::from(profile.display_name.as_str()));
        map.insert("isActive".to_string(), Llsd::from(profile.is_active));
        Llsd::Map(map)
    }
}

impl From<UserProfile> for Llsd {
    fn from(profile: UserProfile) -> Self {
        Llsd::from(&profile)
    }
}

fn main() -> anyhow::Result<()> {
    println!("LLSD Derive Example");
    println!("===================");

    // Example 1: Basic Person conversion
    let person = Person {
        first_name: "John".to_string(),
        last_name: "Doe".to_string(),
        age: 30,
        email: Some("john.doe@example.com".to_string()),
    };

    // Convert struct to LLSD using the Into trait
    let llsd: Llsd = person.clone().into();
    println!("Person as LLSD:");
    println!("{:#?}", llsd);

    // Convert LLSD back to struct using TryFrom
    let restored_person: Person = Person::try_from(&llsd)?;
    println!("\nRestored Person:");
    println!("{:#?}", restored_person);

    assert_eq!(person, restored_person);
    println!("✓ Person conversion successful!");

    // Example 2: UserProfile with camelCase field naming
    let user_profile = UserProfile {
        user_id: 12345,
        display_name: "johndoe".to_string(),
        is_active: true,
    };

    // Convert to LLSD
    let user_llsd: Llsd = user_profile.clone().into();
    println!("\nUser Profile as LLSD (camelCase fields):");
    println!("{:#?}", user_llsd);

    // Convert back
    let restored_profile: UserProfile = UserProfile::try_from(&user_llsd)?;
    println!("\nRestored User Profile:");
    println!("{:#?}", restored_profile);

    assert_eq!(user_profile, restored_profile);
    println!("✓ UserProfile conversion successful!");

    // Example 3: Demonstrate field name transformation
    println!("\nField name transformations:");
    if let Some(map) = user_llsd.as_map() {
        for key in map.keys() {
            println!("  LLSD field: {}", key);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_person_conversion() {
        let person = Person {
            first_name: "Alice".to_string(),
            last_name: "Smith".to_string(),
            age: 25,
            email: None,
        };

        // Convert to LLSD and back
        let llsd: Llsd = person.clone().into();
        let restored: Person = Person::try_from(&llsd).unwrap();
        
        assert_eq!(person, restored);
    }

    #[test]
    fn test_person_with_email() {
        let person = Person {
            first_name: "Bob".to_string(),
            last_name: "Jones".to_string(),
            age: 35,
            email: Some("bob@example.com".to_string()),
        };

        let llsd: Llsd = person.clone().into();
        let restored: Person = Person::try_from(&llsd).unwrap();
        
        assert_eq!(person, restored);
    }

    #[test]
    fn test_user_profile_conversion() {
        let user_profile = UserProfile {
            user_id: 999,
            display_name: "testuser".to_string(),
            is_active: false,
        };

        let llsd: Llsd = user_profile.clone().into();
        let restored: UserProfile = UserProfile::try_from(&llsd).unwrap();
        
        assert_eq!(user_profile, restored);
    }

    #[test]
    fn test_camel_case_field_names() {
        let user_profile = UserProfile {
            user_id: 42,
            display_name: "camelCase".to_string(),
            is_active: true,
        };

        let llsd: Llsd = user_profile.into();
        
        // Verify the field names are in camelCase
        if let Some(map) = llsd.as_map() {
            assert!(map.contains_key("userId"));
            assert!(map.contains_key("displayName"));
            assert!(map.contains_key("isActive"));
            
            // Verify snake_case fields don't exist
            assert!(!map.contains_key("user_id"));
            assert!(!map.contains_key("display_name"));
            assert!(!map.contains_key("is_active"));
        } else {
            panic!("Expected LLSD Map");
        }
    }

    #[test]
    fn test_optional_field() {
        // Test with None email
        let person_no_email = Person {
            first_name: "Test".to_string(),
            last_name: "User".to_string(),
            age: 20,
            email: None,
        };

        let llsd: Llsd = person_no_email.clone().into();
        
        // Verify email field is not present when None
        if let Some(map) = llsd.as_map() {
            assert!(!map.contains_key("email"));
        }

        // Test round trip
        let restored: Person = Person::try_from(&llsd).unwrap();
        assert_eq!(person_no_email, restored);
    }
}
