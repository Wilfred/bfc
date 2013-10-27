(ns brainfrack.core
  (:gen-class))

(defn -main
  "I don't do a whole lot ... yet."
  [& args]
  (println "Hello, World!"))

(defn matching-brackets?
  "Verify that every [ is matched with a ] in the string."
  [string]
  (loop [remaining (seq string) nesting-level 0]
    (cond
     (= remaining '())
     ;; at the end of the string, we shouldn't have any unmatched brackets
     (= nesting-level 0)

     (= (first remaining) \[)
     ;; an open bracket, so increment nesting level and continue
     (recur (rest remaining) (inc nesting-level))

     (= (first remaining) \])
     ;; a close bracket, so decrement the nesting level if we had an open
     (if (<= nesting-level 0)
       ;; unbalanced, we have closed more than we've opened
       false
       (recur (rest remaining) (dec nesting-level)))

     :else
     ;; neither an open nor a close, so recurse on the remainder
     (recur (rest remaining) nesting-level))))
