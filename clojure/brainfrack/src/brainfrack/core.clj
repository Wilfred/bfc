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

(defn eval-program
  "Evaluate the brainfrack program given."
  [program]
  (let [bracket-map (find-matches program)]
    (if (matching-brackets? program)
      ;; evaluate the program
      
      (loop [memory (int-array 30000)
             instructions (char-array program)
             data-index 0
             instruction-index 0]
        ;; terminate when we reach the end of our instructions
        (when-not (>= instruction-index (alength instructions))
          (let [instruction (aget instructions instruction-index)]
            (cond
             
             (= instruction \>)
             (recur memory instructions (inc data-index) (inc instruction-index))

             (= instruction \<)
             (recur memory instructions (dec data-index) (inc instruction-index))

             (= instruction \+)
             (let [old-value (aget memory data-index)]
               (aset memory data-index (inc old-value))
               (recur memory instructions data-index (inc instruction-index)))

             (= instruction \-)
             (let [old-value (aget memory data-index)]
               (aset memory data-index (dec old-value))
               (recur memory instructions data-index (inc instruction-index)))

             (= instruction \.)
             (do
               (print (char (aget memory data-index)))
               (recur memory instructions data-index (inc instruction-index)))

             (= instruction \,)
             (let [new-value (.read *in*)]
               (aset memory data-index new-value)
               (recur memory instructions data-index (inc instruction-index)))

             (= instruction \[)
             ;; jump to the closing bracket if the current value is zero
             (let [current-value (aget memory data-index)]
               (if (= current-value 0)
                 ;; jump to the closing bracket
                 (recur memory instructions data-index (inc (bracket-map instruction-index)))

                 ;; step past the [
                 (recur memory instructions data-index (inc instruction-index))))

             (= instruction \])
             ;; simply jump back to the matching open bracket
             (recur memory instructions data-index ((map-invert bracket-map) instruction-index))))))

      ;; else whinge that this isn't a valid program
      (print "That isn't a valid brainfrack program, check your [ and ] are matched up."))))

(defn -main
  "Read a BF program from stdin and evaluate it."
  []
  ;; FIXME: only reads the first line from stdin
  (eval-program (read-line)))
