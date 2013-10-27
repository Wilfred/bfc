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

(defn find-matches
  "Return a hash-map matching indexes of open brackets to close brackets."
  [program]
  (loop [instructions (seq program)
         match-map {}
         index 0
         stack ()]
    (cond
     ;; return the map when we have no instructions left
     (= instructions '())
     match-map

     ;; given an open bracket, push its index to the stack
     (= (first instructions) \[)
     (recur (rest instructions) match-map (inc index) (conj stack index))

     ;; given a close bracket, pop its open index from the stack and
     ;; add to the match map
     (= (first instructions) \])
     (recur
      (rest instructions)
      (conj match-map {(first stack) index})
      (inc index)
      (rest stack))

     ;; otherwise, it's not an instruction we care about
     :else
     (recur (rest instructions) match-map (inc index) stack))))
