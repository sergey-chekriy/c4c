workspace {
  name "Milestone 3"
  description "Core grammar fixture"
  properties {
    "owner" "Architecture"
  }
  !identifiers hierarchical
  !docs docs
  !adrs adrs
  configuration {
    scope softwareSystem
  }
  model {
    !impliedRelationships false
    archetypes {
      service = element "Service"
    }
    enterprise "Acme" {
      group "Business" {
        group "Retail" {
          customer = person "Customer" {
            description "A customer"
            tag External
            tags "Person,Critical"
            url "https://example.test/customer"
            properties {
              "owner" "Retail"
            }
            perspectives {
              "Security" "Public user"
            }
            !docs docs/customer
            !adrs adrs/customer
          }
          bank = softwareSystem "Bank" {
            description "Banking system"
            tags Core
            group "Applications" {
              api = container "API" "Backend" "Rust" {
                logic = component "Logic" "Business logic" "Rust"
                !components src
              }
            }
          }
        }
      }
    }
    queue = element "Queue" "Broker" "Events" "NATS" "Infrastructure" {
      technology JetStream
      instanceOf bank
    }
    customer -> bank "Uses" {
      description "Uses securely"
      technology HTTPS
      tag Critical
      tags External
      url "https://example.test/relationship"
      properties {
        "sla" "99.9"
      }
      perspectives {
        "Security" "TLS"
      }
    }
    !extend bank
    !ref customer
    !element bank
    !elements *
    !relationship customer bank
    !relationships *
    !components src
  }
}
