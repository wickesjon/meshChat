import XCTest

final class MeshUITests: XCTestCase {
    @MainActor func testProductionFeatureIntegration() {
        let app = XCUIApplication(); app.launchArguments = ["--integration"]; app.launch()
        let result = app.staticTexts["integration-result"]
        XCTAssertTrue(result.waitForExistence(timeout: 90))
        XCTAssertEqual(result.label, "MC035 integration PASS")
    }
    @MainActor func testNativeScreensAndConfirmations() {
        let app = XCUIApplication(); app.launch()
        let nickname = app.textFields["Nickname"]
        XCTAssertTrue(nickname.waitForExistence(timeout: 20)); nickname.tap(); nickname.typeText("Synthetic Alice")
        app.buttons["Continue"].tap(); XCTAssertTrue(app.staticTexts.containing(NSPredicate(format: "label CONTAINS 'Bluetooth'")).firstMatch.exists)
        app.buttons["Continue"].tap(); app.buttons["Create my local identity"].tap()
        XCTAssertTrue(app.buttons["Join a word-triple channel"].waitForExistence(timeout: 20))
        app.buttons["Join a word-triple channel"].tap()
        XCTAssertTrue(app.staticTexts["Anyone with these words can read this channel. It is not encrypted."].waitForExistence(timeout: 5))
        app.buttons["Join channel"].tap()
        XCTAssertTrue(app.textFields["Message"].waitForExistence(timeout: 5))
        attach(app, "channel-plaintext")
        app.buttons["Back"].tap(); app.buttons["Settings"].tap()
        XCTAssertTrue(app.buttons["Beacon Mode · unavailable on iOS"].exists)
        XCTAssertFalse(app.buttons["Beacon Mode · unavailable on iOS"].isEnabled)
        attach(app, "settings")
        app.buttons["Cancel"].firstMatch.tap()
        app.tabBars.buttons["Friends"].tap(); app.buttons["My friend code"].tap()
        XCTAssertTrue(app.buttons["Copy link"].waitForExistence(timeout: 5))
        attach(app, "public-friend-qr")
        app.buttons["Done"].firstMatch.tap()
        app.tabBars.buttons["Messages"].tap()
        XCTAssertTrue(app.staticTexts.containing(NSPredicate(format: "label CONTAINS 'no plaintext fallback'")).firstMatch.exists)
    }
    @MainActor private func attach(_ app: XCUIApplication, _ name: String) {
        let attachment = XCTAttachment(screenshot: app.screenshot()); attachment.name = name; attachment.lifetime = .keepAlways; add(attachment)
    }
}
