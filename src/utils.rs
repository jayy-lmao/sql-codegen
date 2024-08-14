use crate::models::TableColumn;

pub(crate) fn to_snake_case(input: &str) -> String {
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

pub fn generate_struct_code(table_name: &str, rows: &Vec<TableColumn>) -> String {
    let struct_name = to_pascal_case(table_name);
    let mut struct_code = String::new();

    struct_code.push_str("#![allow(dead_code)]\n");
    struct_code.push_str("// Generated with sql-gen\n// https://github.com/jayy-lmao/sql-gen\n\n");
    struct_code.push_str("#[derive(sqlx::FromRow, Debug)]\n");
    struct_code.push_str(&format!("pub struct {} {{\n", struct_name));

    for row in rows {
        if row.table_name == table_name {
            let column_name = to_snake_case(&row.column_name);
            let mut data_type = convert_data_type(&row.udt_name);
            let optional_type = format!("Option<{}>", data_type);
            if row.is_nullable {
                data_type = optional_type;
            }

            struct_code.push_str(&format!("  pub {}: {},\n", column_name, data_type));
        }
    }
    struct_code.push_str("}\n");

    struct_code
}

pub fn convert_data_type(data_type: &str) -> String {
    if data_type.to_lowercase().contains("char(") {
        return "String".to_string();
    }
    if data_type.starts_with("_") {
        let array_of_type = convert_data_type(&data_type[1..]);
        let vec_type = format!("Vec<{}>", array_of_type);
        return vec_type;
    }

    match data_type {
        "bool" | "boolean" => "bool",
        "bytea" => "Vec<u8>", // is this right?
        "char" | "bpchar" | "character" => "String",
        "date" => "chrono::NaiveDate",
        "float4" | "real" => "f32",
        "float8" | "double precision" => "f64",
        "int2" | "smallint" | "smallserial" => "i16",
        "int4" | "int" | "serial" => "i32",
        "int8" | "bigint" | "bigserial" => "i64",
        "void" => "()",
        "jsonb" | "json" => "serde_json::Value",
        "text" | "varchar" | "name" | "citext" => "String",
        "time" => "chrono::NaiveTime",
        "timestamp" => "chrono::NaiveDateTime",
        "timestamptz" => "chrono::DateTime<chrono::Utc>",
        "uuid" => "uuid::Uuid",
        "cube" => "sqlx::postgres::types::PgCube",
        _ => panic!("Unknown type: {}", data_type),
    }
    .to_string()
}

pub fn convert_data_type_from_pg(data_type: &str) -> String {
    if data_type.contains("Json<") {
        return "jsonb".to_string();
    }
    if data_type.contains("Vec<") {
        let array_type = convert_data_type_from_pg(&data_type[4..data_type.len() - 1]);
        return format!("{}[]", array_type);
    }
    match data_type {
        "i64" => "int8",
        "i32" => "int4",
        "i16" => "int2",
        "String" => "text",
        "serde_json::Value" => "jsonb",
        "chrono::DateTime<chrono::Utc>" => "timestamptz",
        "chrono::NaiveDateTime" => "timestamp",
        "DateTime<Utc>" => "timestamptz",
        "chrono::NaiveDate" => "date",
        "f32" => "float4",
        "f64" => "float8",
        "uuid::Uuid" => "uuid",
        "bool" => "boolean",
        "Vec<u8>" => "bytea", // is this right ?
        _ => panic!("Unknown type: {}", data_type),
    }
    .to_string()
}

fn generate_query_code(_row: &TableColumn) -> String {
    // ... (implementation of generate_query_code)
    // query_code
    todo!()
}

pub fn parse_struct_fields(struct_code: &str) -> Vec<(String, String, bool)> {
    let lines = struct_code.lines();
    let mut fields = Vec::new();

    for line in lines {
        let trimmed_line = line.trim();
        if !trimmed_line.starts_with("pub") {
            continue;
        }

        let parts: Vec<&str> = trimmed_line.split(": ").collect();
        if parts.len() != 2 {
            continue;
        }

        let field = parts[0].trim().trim_start_matches("pub").trim();
        //let data_type_optional = parts[1].trim().trim_end_matches(",").trim();
        let mut is_nullable = false;

        let data_type = if parts[1].trim().starts_with("Option") {
            is_nullable = true;
            parts[1]
                .trim()
                .trim_start_matches("Option<")
                .trim_end_matches(">,")
        } else {
            parts[1].trim().trim_end_matches(',')
        };

        fields.push((field.to_owned(), data_type.to_owned(), is_nullable));
    }

    fields
}

#[cfg(test)]
mod tests {
    // ... (unit tests can be defined here)
}

pub fn to_pascal_case(input: &str) -> String {
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
