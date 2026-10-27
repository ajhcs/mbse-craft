#!/usr/bin/env python3
"""Input fidelity contract; imports never launch a display/app."""
import importlib.util
from pathlib import Path
import unittest
spec=importlib.util.spec_from_file_location('driver',Path(__file__).with_name('verify-m1-native.py'))
driver=importlib.util.module_from_spec(spec);spec.loader.exec_module(driver)
class TextKeys(unittest.TestCase):
 def test_question_and_number_sign_require_named_shifted_keysyms(self):
  self.assertEqual(driver.text_keys('10?#'),[('1',False),('0',False),('question',True),('numbersign',True)])
 def test_all_printable_ascii_is_admitted_with_one_event_pair_each(self):
  text=''.join(chr(c) for c in range(32,127));self.assertEqual(len(driver.text_keys(text)),95)
 def test_uppercase_is_shifted_and_plain_letters_and_path_symbols_remain_supported(self):
  self.assertEqual(driver.text_keys('Aa/._'),[('a',True),('a',False),('slash',False),('period',False),('underscore',True)])
 def test_unsupported_text_refuses_before_returning_a_partial_event_sequence(self):
  for text in ['valid\nrest','valid\t','valid\x00','validé','😀']:
   with self.assertRaisesRegex(ValueError,'NATIVE_TEXT_UNSUPPORTED'):driver.text_keys(text)
if __name__=='__main__':unittest.main()
