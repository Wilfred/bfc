(ns brainfrack.core
  (:use [clojure.set :only (map-invert)])
  (:gen-class))

(defn matching-brackets?
  "Verify that every [ is matched with a ] in the string."
  [string]
  (loop [remaining (seq string) nesting-level 0]
    (cond
     (= remaining '())
     ;; at the end of the string, we shouldn't have any unmatched brackets
     (zero? nesting-level)

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

(defn eval-program
  "Evaluate the brainfrack program given."
  [program]
  (let [bracket-map (find-matches program)
        memory (ref (vec (repeat 30000 0)))
        instructions (vec program)
        data-index (ref 0)
        instruction-index (ref 0)]
    (cond
     ;; empty program, nothing to do
     (empty? (seq program))
     nil

     ;; invalid program, just whinge and terminate
     (not (matching-brackets? program))
     (println "That isn't a valid brainfrack program, check your [ and ] are matched up.")

     ;; evaluate the program, terminating when we reach the end of our instructions
     :else
     (while (< @instruction-index (count instructions))
      (let [instruction (instructions @instruction-index)]
        (cond
         
         (= instruction \>)
         (dosync
          (alter data-index inc)
          (alter instruction-index inc))

         (= instruction \<)
         (dosync
          (alter data-index dec)
          (alter instruction-index inc))

         (= instruction \+)
         (dosync
          (alter memory #(update-in % [@data-index] inc))
          (alter instruction-index inc))

         (= instruction \-)
         (dosync
          (alter memory #(update-in % [@data-index] dec))
          (alter instruction-index inc))

         (= instruction \.)
         (dosync
          (print (char (@memory @data-index)))
          (alter instruction-index inc))

         (= instruction \,)
         (dosync
          (alter memory #(assoc % @data-index (.read *in*)))
          (alter instruction-index inc))

         (= instruction \[)
         ;; jump to the closing bracket if the current value is zero
         (if (zero? (@memory @data-index))
           ;; jump to character after closing bracket
           (dosync
            (ref-set instruction-index (inc (bracket-map @instruction-index))))

           ;; step past the [
           (dosync
            (alter instruction-index inc)))

         (= instruction \])
         ;; simply jump back to the matching open bracket
         (dosync
          (ref-set instruction-index ((map-invert bracket-map) @instruction-index)))

         ;; ignore all other characters
         :else
         (dosync
          (alter instruction-index inc))))))))

(defn -read-all-stdin
  "Read all the text from stdin and return as a string."
  []
  (apply str (line-seq (java.io.BufferedReader. *in*))))

(defn -main
  "Read a BF program from stdin and evaluate it."
  []
  (eval-program (-read-all-stdin)))
