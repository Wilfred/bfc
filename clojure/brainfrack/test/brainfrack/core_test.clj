(ns brainfrack.core-test
  (:use clojure.test
        [brainfrack.core :only (matching-brackets?)]))

(deftest matching-brackets
  (testing "Verify well-bracketed expressions pass."
    (is (matching-brackets? ""))
    (is (matching-brackets? "[]"))
    (is (matching-brackets? "[][]"))
    (is (matching-brackets? "[+[- ]<> foo ]")))
  (testing "Verify well-bracketed expressions pass."
    (is (not (matching-brackets? "[")))
    (is (not (matching-brackets? "]")))
    (is (not (matching-brackets? "[]]")))
    (is (not (matching-brackets? "][")))))
