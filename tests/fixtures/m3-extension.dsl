workspace extends "m3-extension-base.dsl" {
  model {
    derivedSystem = softwareSystem "Derived System" {
      api = container "API"
    }
    baseUser -> derivedSystem "Uses"
  }
}
