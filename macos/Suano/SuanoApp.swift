//
//  SuanoApp.swift
//  Suano — macOS AI writing assistant
//


import SwiftUI
import AppKit

@main
struct SuanoApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) var appDelegate

    var body: some Scene {
        Settings {
            EmptyView()
        }
    }
}
