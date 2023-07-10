use clap::{App, Arg, SubCommand};
use sqlx::postgres::{PgPool, PgPoolOptions, PgRow};
use sqlx::Row;
use std::fs;
use std::path::PathBuf;

#[derive(sqlx::FromRow)]
struct TableColumn {
    table_name: String,
    column_name: String,
    udt_name: String,
    is_nullable: bool
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = App::new("SQL Gen")
        .subcommand(
            SubCommand::with_name("generate")
                .about("Generate structs and queries for tables")
                .arg(
                    Arg::with_name("output")
                        .short('o')
                        .long("output")
                        .value_name("FOLDER")
                        .help("Sets the output folder")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::with_name("database")
                        .short('d')
                        .long("database")
                        .value_name("URL")
                        .help("Sets the database connection URL")
                        .takes_value(true)
                        .required(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("migrate")
                .about("Generate SQL migrations based on struct differences")
                .arg(
                    Arg::with_name("include")
                        .short('i')
                        .long("include")
                        .value_name("FOLDER")
                        .help("Sets the folder containing existing struct files")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::with_name("output")
                        .short('o')
                        .long("output")
                        .value_name("FOLDER")
                        .help("Sets the output folder for migrations")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::with_name("database")
                        .short('d')
                        .long("database")
                        .value_name("URL")
                        .help("Sets the database connection URL")
                        .takes_value(true)
                        .required(true),
                ),
        )
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("generate") {
        let output_folder = matches.value_of("output").unwrap();
        let database_url = matches.value_of("database").unwrap();

        // Connect to the Postgres database
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        // Get all tables from the database
        let query = "
            SELECT table_name, column_name, udt_name, is_nullable = 'YES' as is_nullable
            FROM information_schema.columns
            WHERE table_schema = 'public' ORDER BY table_name, ordinal_position
        ";

        let rows = sqlx::query_as::<_, TableColumn>(query)
            .fetch_all(&pool)
            .await?;

        // Create the output folder if it doesn't exist
        fs::create_dir_all(output_folder)?;

        // let tables_duplicated = rows.iter().map(|row| row.table_name.clone()).collect::<Vec<String>>();
let mut unique = std::collections::BTreeSet::new();
for row in &rows {
    unique.insert(row.table_name.clone());
}
let tables = unique.into_iter().collect::<Vec<String>>();



        println!("Outputting tables: {:?}", tables);

        // Generate structs and queries for each table
        for table in tables {
            // Generate the struct code based on the row
            let struct_code = generate_struct_code(&table, &rows);

            // Generate the query code based on the row
            // let query_code = generate_query_code(&row);

            let struct_file_path = format!("{}/{}.rs", output_folder, to_snake_case(&table));
            fs::write(struct_file_path, struct_code)?;

            // Write the query code to a file
            // let query_file_path = format!("{}/{}_query.rs", output_folder, row.table_name);
            // fs::write(query_file_path, query_code)?;
        }
    } else if let Some(matches) = matches.subcommand_matches("migrate") {
        let include_folder = matches.value_of("include").unwrap();
        let output_folder = matches.value_of("output").unwrap();
        let database_url = matches.value_of("database").unwrap();

        // Connect to the Postgres database
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        // Read existing struct files from the include folder
        let existing_files = fs::read_dir(include_folder)?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .collect::<Vec<PathBuf>>();

        // Create the output folder if it doesn't exist
        fs::create_dir_all(output_folder)?;

        // Generate migrations for struct differences
        for file_path in existing_files {
            // Parse the struct name from the file name
            let file_name = file_path.file_stem().unwrap().to_string_lossy().to_string();
            let struct_name = file_name;

            // Read the struct code from the file
            let struct_code = fs::read_to_string(&file_path)?;

            // Check if the struct fields differ from the database
            let migration_code = generate_migration_code(&struct_name, struct_code, &pool).await?;

            // Generate a timestamp and migration name
            let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
            let migration_name = format!("{}_{}.sql", timestamp, struct_name);

            // Write the migration code to a file
            let migration_file_path = format!("{}/{}", output_folder, migration_name);
            fs::write(migration_file_path, migration_code)?;
        }
    }
    Ok(())
}

async fn generate_migration_code(
    struct_name: &str,
    struct_code: String,
    pool: &PgPool,
) -> Result<String, Box<dyn std::error::Error>> {
    let table_name_lower = struct_name.to_lowercase();
    let table_name_upper = to_pascal_case(&struct_name);

    // Get the column names and data types from the struct code
    let fields = parse_struct_fields(&struct_code);

    // Query the database for column information
    let query_lower = format!(
        "SELECT column_name, udt_name, is_nullable
         FROM information_schema.columns
         WHERE table_name = '{}'",
        table_name_lower,
    );

    let existing_columns_lower: Vec<(String, String, String)> = sqlx::query_as(query_lower.as_str())
        .fetch_all(pool)
        .await?;

    // Query the database for column information
    let query_upper = format!(
        "SELECT column_name, udt_name, is_nullable
         FROM information_schema.columns
         WHERE table_name = '{}'",
        table_name_upper,
    );

    let existing_columns_upper: Vec<(String, String, String)> = sqlx::query_as(query_upper.as_str())
        .fetch_all(pool)
        .await?;


    let (table_name, existing_columns) = match (!existing_columns_lower.is_empty(), !existing_columns_upper.is_empty()) {
(true, _) => (table_name_lower, existing_columns_lower),
(_, true) => (table_name_upper, existing_columns_upper),
_ => { panic!("Table does not exist for {} or {}", table_name_lower, table_name_upper); }
    };

    println!("Struct: {:?}", struct_name);
    println!("Existing Columns: {:?}", existing_columns);
    println!("Fields Columns: {:?}", fields);
    

    // Compare existing columns with struct fields
    let mut migration_statements = Vec::<String>::new();

    for (column_name, data_type, is_nullable) in &fields {
        println!("trying {} {} ", column_name, data_type);
        let matching_column = existing_columns.iter().find(|(col_name, _, _)| col_name == column_name);

        if let Some((_, existing_type, existing_nullable)) = matching_column {
            // Compare data types and nullability
            if data_type != existing_type || is_nullable != existing_nullable {
                let alter_table = format!("ALTER TABLE {}", table_name);

                // Generate appropriate column definition
                let column_definition = convert_data_type_from_pg(data_type);

                // Generate the ALTER TABLE statement
                let nullable_keyword = if is_nullable == "YES" {
                    "DROP NOT NULL"
                } else {
                    "SET NOT NULL"
                };

                let migration_statement = format!(
                    "{} ALTER COLUMN {} TYPE {}, {}",
                    alter_table, column_name, column_definition, nullable_keyword
                );

                migration_statements.push(migration_statement);
            }
        } else {
            let alter_table = format!("ALTER TABLE {}", table_name);
                            let column_definition = convert_data_type_from_pg(data_type);

            let nullable_keyword = if is_nullable == "YES" {
                "NULL"
            } else {
                "NOT NULL"
            };
            let migration_statement = format!(
                "{} ADD COLUMN {} {} {}",
                alter_table, column_name, column_definition, nullable_keyword
            );
            migration_statements.push(migration_statement);
        }
    }

    // Compare existing columns with struct fields to identify removed columns
    let removed_columns: Vec<&(String, _, _)> = existing_columns
        .iter()
        .filter(|(col_name, _, _)| !fields.iter().any(|(field_name, _, _)| field_name == col_name))
        .collect();

    for (column_name, _, _) in removed_columns {
        let alter_table = format!("ALTER TABLE {}", table_name);
        let drop_column = format!("DROP COLUMN {}", column_name);
        let migration_statement = format!("{} {}", alter_table, drop_column);
        migration_statements.push(migration_statement);
    }

    // Generate the full migration code
    let migration_code = if !migration_statements.is_empty() {
        let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
        let migration_name = format!("{}_{}.sql", timestamp, struct_name);

        let migration_statements_code = migration_statements.join(";\n");

        format!(
            "-- Migration generated for struct: {}\n{}\n",
            struct_name, migration_statements_code
        )
    } else {
        String::new()
    };

    Ok(migration_code)
}

fn generate_struct_code(table_name: &str, rows: &Vec<TableColumn>) -> String {
    let struct_name = to_pascal_case(table_name);
    let mut struct_code = format!("#[derive(sqlx::FromRow)]\n");
    struct_code.push_str(&format!("pub struct {} {{\n", struct_name));

    for row in rows {
        if row.table_name == table_name {
    let column_name = to_snake_case(&row.column_name);
    let mut data_type = convert_data_type(&row.udt_name);
    let optional_type = format!("Option<{}>", data_type);
  if row.is_nullable {
    data_type = optional_type.as_str();
  } 
    
    struct_code.push_str(&format!(" pub {}: {},\n", column_name, data_type));
            }    }
    struct_code.push_str("}\n");

    struct_code
}

fn convert_data_type(data_type: &str) -> &str {
    match data_type {
        "int8" => "i64",
        "int4" => "i32",
        "int2" => "i16",
        "text" => "String",
        "varchar" => "String",
        "jsonb" => "sqlx::Json",
        "timestamptz" => "chrono::DateTime<chrono::Utc>",
        "date" => "chrono::NaiveDate",
        "float4" => "f32",
        "float8" => "f64",
        "uuid" => "uuid::Uuid",
        "boolean" => "bool",
        "bytea" => "Vec<u8>", // is this right?
        _ => panic!("Unknown type: {}",data_type),
    }
}


fn convert_data_type_from_pg(data_type: &str) -> &str {
    match data_type {
        "i64" => "int8",
        "i32" => "int4",
        "i16" => "int2",
        "String" => "text",
        "String" => "varchar",
        "sqlx::Json" => "jsonb",
        "chrono::DateTime<chrono::Utc>" => "timestamptz",
        "chrono::NaiveDate" => "date",
        "f32" => "float4",
        "f64" => "float8",
        "uuid::Uuid" => "uuid",
        "bool" => "boolean",
        "Vec<u8>" => "bytea", // is this right ?
        _ => panic!("Unknown type: {}",data_type),
    }
}



fn generate_query_code(row: &TableColumn) -> String {
    // ... (implementation of generate_query_code)
    // query_code
    todo!()
}

// fn parse_struct_fields(struct_code: &str) -> Vec<(String, String, String)> {
//     let struct_regex = regex::Regex::new(r"pub\s+(?P<field>\w+):\s+(?P<type>\w+),?").unwrap();
//     let captures_iter = struct_regex.captures_iter(struct_code);

//     let mut fields = Vec::new();

//     for captures in captures_iter {
//         if let (Some(field), Some(data_type)) = (captures.name("field"), captures.name("type")) {
//             fields.push((field.as_str().to_owned(), data_type.as_str().to_owned(), "".to_owned()));
//         }
//     }

//     fields
// }

fn parse_struct_fields(struct_code: &str) -> Vec<(String, String, String)> {
    let lines = struct_code.lines();
    let mut fields = Vec::new();

    for line in lines {
        let trimmed_line = line.trim();
        if !trimmed_line.starts_with("pub") {
            continue;
        }

        let parts: Vec<&str> = trimmed_line.split(":").collect();
        if parts.len() != 2 {
            continue;
        }

        let field = parts[0].trim().trim_start_matches("pub").trim();
        let data_type_optional = parts[1].trim().trim_end_matches(",").trim();
        let mut is_nullable = String::from("NO");

        let data_type = if data_type_optional.starts_with("Option") {
            is_nullable = String::from("YES");
           data_type_optional.trim_start_matches("Option<").trim_end_matches(">"
           )
        } else { data_type_optional };

        fields.push((field.to_owned(), data_type.to_owned(), is_nullable));
    }

    fields
}


#[cfg(test)]
mod tests {
    // ... (unit tests can be defined here)
}

fn to_pascal_case(input: &str) -> String {
    let mut output = String::new();
    let mut capitalize_next = true;

    for c in input.chars() {
        if c.is_ascii_alphanumeric() {
            if capitalize_next {
                output.extend(c.to_uppercase());
                capitalize_next = false;
            } else {
                output.push(c);
            }
        } else {
            capitalize_next = true;
        }
    }

    output
}

fn to_snake_case(input: &str) -> String {
    let mut output = String::new();
    let mut prev_is_uppercase = false;

    for c in input.chars() {
        if c.is_ascii_uppercase() {
            if !output.is_empty() && !prev_is_uppercase {
                output.push('_');
            }
            output.extend(c.to_lowercase());
            prev_is_uppercase = true;
        } else {
            output.push(c);
            prev_is_uppercase = false;
        }
    }

    output
}
