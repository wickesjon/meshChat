"""Generate an isolated test host from the production target and run native UI tests.

No synthetic protection or radio code is linked into the shipping application.
All generated project/build/results/package caches remain under .work/.
"""
from pathlib import Path
import json
import os
import platform
import plistlib
import subprocess
import uuid

ROOT = Path(__file__).resolve().parents[3]
WORK = ROOT / ".work/ios-ui"
TESTS = ROOT / "tests/integration/ios-ui"


def run(*args):
    return subprocess.run(args, cwd=ROOT, env=ENV, check=True, text=True, capture_output=True).stdout


if __name__ == "__main__":
    if platform.system() != "Darwin":
        raise RuntimeError("Native Mac/Xcode required")
    WORK.mkdir(parents=True, exist_ok=True)
    ENV = os.environ.copy()
    ENV["TMPDIR"] = str(ROOT / ".work/tmp")
    source = ROOT / "src/ios"
    project = WORK / "Harness.xcodeproj"
    project.mkdir(exist_ok=True)
    original = source / "MeshChat.xcodeproj/project.pbxproj"
    data = json.loads(run("plutil", "-convert", "json", "-o", "-", str(original)))
    objects = data["objects"]
    def add(value):
        key = uuid.uuid4().hex[:24].upper()
        objects[key] = value
        return key
    sources = objects["A00000000000000000000007"]["files"]
    for obj in objects.values():
        if obj.get("isa") == "PBXFileReference" and obj.get("sourceTree") == "<group>":
            obj["path"] = str((source / obj["path"]).resolve())
            obj["sourceTree"] = "<absolute>"
        if obj.get("isa") == "XCBuildConfiguration":
            settings = obj["buildSettings"]
            for key, value in list(settings.items()):
                if isinstance(value, str):
                    settings[key] = value.replace("$(SRCROOT)/../../.work", str(ROOT / ".work"))
            if "INFOPLIST_FILE" in settings:
                settings["INFOPLIST_FILE"] = str(source / "MeshChat/Info.plist")
                settings["PRODUCT_BUNDLE_IDENTIFIER"] = "org.meshchat.uitesthost"
    # Replace only the app entry point; all production feature sources stay linked.
    objects["A00000000000000000000005"]["path"] = str(TESTS / "HarnessApp.swift")
    for name in ["TestEnvironment.swift", "FeatureChecks.swift"]:
        ref = add({"isa": "PBXFileReference", "lastKnownFileType": "sourcecode.swift", "path": str(TESTS / name), "sourceTree": "<absolute>"})
        sources.append(add({"isa": "PBXBuildFile", "fileRef": ref}))
    fixtures = ROOT / ".work/mc030/fixtures/events.tsv"
    if not fixtures.exists():
        raise RuntimeError("Generate disposable organizer fixtures first")
    ref = add({"isa": "PBXFileReference", "lastKnownFileType": "text", "path": str(fixtures), "sourceTree": "<absolute>"})
    objects["A00000000000000000000009"]["files"].append(add({"isa": "PBXBuildFile", "fileRef": ref}))
    ref = add({"isa": "PBXFileReference", "lastKnownFileType": "sourcecode.swift", "path": str(TESTS / "UITests.swift"), "sourceTree": "<absolute>"})
    build = add({"isa": "PBXBuildFile", "fileRef": ref})
    phase = add({"isa": "PBXSourcesBuildPhase", "buildActionMask": "2147483647", "files": [build], "runOnlyForDeploymentPostprocessing": "0"})
    product = add({"isa": "PBXFileReference", "explicitFileType": "wrapper.cfbundle", "path": "MeshUITests.xctest", "sourceTree": "BUILT_PRODUCTS_DIR"})
    configs = []
    for name in ["Debug", "Release"]:
        configs.append(add({"isa": "XCBuildConfiguration", "name": name, "buildSettings": {
            "GENERATE_INFOPLIST_FILE": "YES", "PRODUCT_BUNDLE_IDENTIFIER": "org.meshchat.uitesthost.tests",
            "PRODUCT_NAME": "$(TARGET_NAME)", "TEST_TARGET_NAME": "MeshChat", "TARGETED_DEVICE_FAMILY": "1,2",
            "SWIFT_VERSION": "6.0", "IPHONEOS_DEPLOYMENT_TARGET": "15.0", "CODE_SIGNING_ALLOWED": "NO",
        }}))
    config = add({"isa": "XCConfigurationList", "buildConfigurations": configs, "defaultConfigurationIsVisible": "0", "defaultConfigurationName": "Debug"})
    dependency = add({"isa": "PBXTargetDependency", "target": "A00000000000000000000004"})
    target = add({"isa": "PBXNativeTarget", "name": "MeshUITests", "productName": "MeshUITests", "productType": "com.apple.product-type.bundle.ui-testing",
                  "productReference": product, "buildConfigurationList": config, "buildPhases": [phase], "buildRules": [], "dependencies": [dependency]})
    objects["A00000000000000000000001"]["targets"].append(target)
    objects["A00000000000000000000003"]["children"].append(product)
    (project / "project.pbxproj").write_bytes(plistlib.dumps(data))
    schemes = project / "xcshareddata/xcschemes"
    schemes.mkdir(parents=True, exist_ok=True)
    def reference(identifier, name, product):
        return f'<BuildableReference BuildableIdentifier="primary" BlueprintIdentifier="{identifier}" BuildableName="{product}" BlueprintName="{name}" ReferencedContainer="container:Harness.xcodeproj"/>'
    app = reference("A00000000000000000000004", "MeshChat", "MeshChat.app")
    tests = reference(target, "MeshUITests", "MeshUITests.xctest")
    (schemes / "Harness.xcscheme").write_text(f'''<?xml version="1.0" encoding="UTF-8"?>
<Scheme version="1.3"><BuildAction buildImplicitDependencies="YES"><BuildActionEntries>
<BuildActionEntry buildForTesting="YES" buildForRunning="YES">{app}</BuildActionEntry>
<BuildActionEntry buildForTesting="YES">{tests}</BuildActionEntry>
</BuildActionEntries></BuildAction><TestAction buildConfiguration="Debug" selectedDebuggerIdentifier="Xcode.DebuggerFoundation.Debugger.LLDB" selectedLauncherIdentifier="Xcode.IDEFoundation.Launcher.LLDB"><Testables><TestableReference skipped="NO">{tests}</TestableReference></Testables></TestAction></Scheme>''')
    devices = json.loads(run("xcrun", "simctl", "list", "devices", "available", "--json"))["devices"]
    candidates = [d for runtime, group in devices.items() if "iOS" in runtime for d in group if "iPhone" in d["name"]]
    if not candidates:
        raise RuntimeError("No available iPhone simulator")
    device = candidates[0]
    result = WORK / ("results-" + uuid.uuid4().hex + ".xcresult")
    command = ["xcodebuild", "-project", str(project), "-scheme", "Harness", "-destination", "id=" + device["udid"],
               "-derivedDataPath", str(WORK / "derived"), "-clonedSourcePackagesDirPath", str(ROOT / ".work/swift-packages"),
               "-resultBundlePath", str(result), "CLANG_MODULE_CACHE_PATH=" + str(ROOT / ".work/clang-module-cache"),
               "CODE_SIGNING_ALLOWED=NO", "test"]
    print("Native synthetic UI/integration:", device["name"], device["udid"], flush=True)
    with (WORK / "native.log").open("w") as log:
        completed = subprocess.run(command, cwd=ROOT, env=ENV, stdout=log, stderr=subprocess.STDOUT)
    if completed.returncode:
        print((WORK / "native.log").read_text()[-18000:])
        raise SystemExit(completed.returncode)
    print("MC035 native UI/integration passed:", result)
