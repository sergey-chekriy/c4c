workspace "Internet Banking" "Milestone 1 example" {

  !identifiers hierarchical

  model {
    customer = person "Customer" "A bank customer"

    bank = softwareSystem "Internet Banking System" "Allows customers to view accounts and make payments" {
      web = container "Web Application" "Customer-facing SPA" "React"
      api = container "API Application" "Backend API" "Rust"
      db = container "Database" "Stores audit/profile data" "PostgreSQL"
    }
    
    customer -> bank.web "Uses"
    bank.web -> bank.api "Calls" "HTTPS/JSON"
    bank.api -> bank.db "Reads/writes" "SQL"
  }

  views {
    systemContext bank "system-context" "System Context diagram" {
      include *
      autolayout lr
      title "Internet Banking - System Context"
    }

    container bank "container" "Container diagram" {
      include *
      autolayout lr
      title "Internet Banking - Containers"
    }
  }
}
