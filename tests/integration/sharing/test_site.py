"""Local template checks; these do not establish production domain ownership."""
import importlib.util
import json
from pathlib import Path
import plistlib
import tempfile
import unittest
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location('share_build', ROOT/'src/share-site/build.py')
builder = importlib.util.module_from_spec(spec)
spec.loader.exec_module(builder)


class SiteTests(unittest.TestCase):
    def setUp(self):
        (ROOT/'.work/mc031').mkdir(parents=True, exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(dir=ROOT/'.work/mc031')
        self.addCleanup(self.temp.cleanup)
        self.output = Path(self.temp.name)
        self.config = json.loads((ROOT/'src/share-site/config.example.json').read_text())

    def test_missing_ids_grant_no_association_and_no_fake_stores(self):
        builder.build(self.config, self.output)
        self.assertEqual([], json.loads((self.output/'.well-known/assetlinks.json').read_text()))
        self.assertEqual([], json.loads((self.output/'.well-known/apple-app-site-association').read_text())['applinks']['details'])
        self.assertIn('"android_store_url": null', (self.output/'config.mjs').read_text())

    def test_configured_fixture_matches_native_templates(self):
        self.config.update(domain='share.example', android_package='org.example.meshchat',
                           android_sha256=[':'.join(['AB']*32)], apple_app_id='ABCDEFGHIJ.org.example.meshchat',
                           android_store_url='https://play.google.com/store/apps/details?id=org.example.meshchat',
                           ios_store_url='https://apps.apple.com/app/id123456789')
        builder.build(self.config, self.output)
        android = json.loads((self.output/'.well-known/assetlinks.json').read_text())[0]
        self.assertEqual(self.config['android_package'], android['target']['package_name'])
        self.assertEqual(self.config['android_sha256'], android['target']['sha256_cert_fingerprints'])
        apple = json.loads((self.output/'.well-known/apple-app-site-association').read_text())['applinks']['details'][0]
        self.assertEqual(self.config['apple_app_id'], apple['appID'])
        self.assertEqual(['/j/*', '/friend/*'], apple['paths'])
        entitlements = plistlib.loads((self.output/'native-templates/AssociatedDomains.entitlements').read_bytes())
        self.assertEqual(['applinks:share.example'], entitlements['com.apple.developer.associated-domains'])
        manifest = ET.parse(self.output/'native-templates/AndroidAppLinks.xml').getroot()
        ns = '{http://schemas.android.com/apk/res/android}'
        self.assertEqual('true', manifest.attrib[ns+'autoVerify'])
        self.assertEqual(['/j/', '/friend/'], [e.attrib[ns+'pathPrefix'] for e in manifest.findall('data') if ns+'pathPrefix' in e.attrib])

    def test_bad_configuration_refuses(self):
        for key, value in [('domain', 'example.org/path'), ('android_package', '<injection>'),
                           ('android_sha256', ['AB:CD']), ('apple_app_id', 'invalid'),
                           ('android_store_url', 'https://play.google.com.evil/store/apps/details'),
                           ('ios_store_url', 'javascript:alert(1)')]:
            with self.subTest(key=key):
                with self.assertRaises(ValueError):
                    builder.build(dict(self.config, **{key: value}), self.output)

    def test_output_stays_in_repository_ignored_work(self):
        with self.assertRaises(ValueError):
            builder.build(self.config, ROOT/'src/share-site/generated')

    def test_page_limits_network_and_native_scope(self):
        html = (ROOT/'src/share-site/index.html').read_text()
        self.assertIn('connect-src \'none\'', html)
        self.assertIn('content="no-referrer"', html)
        self.assertNotIn('https://', html)
        manifest = (ROOT/'src/android/app/src/main/AndroidManifest.xml').read_text()
        self.assertNotIn('android.permission.INTERNET', manifest)
        self.assertIn('android:exported="false" android:grantUriPermissions="true"', manifest)
        paths = ET.parse(ROOT/'src/android/app/src/main/res/xml/share_paths.xml').getroot()
        self.assertEqual([('cache-path', 'share-qr/')], [(e.tag, e.attrib['path']) for e in paths])


if __name__ == '__main__':
    unittest.main()
