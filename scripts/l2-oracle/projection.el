;;; projection.el --- L2 oracle schema-v1 projection (Story 2.7, LD-45) -*- lexical-binding: t; -*-

;; Projects an org file's `org-element-parse-buffer' tree onto the
;; version-stable "l2-projection-v1" schema and prints it as JSON on
;; stdout. Raw org-element dumps are NOT comparable across Org versions
;; (9.6 vs 9.7 `:standard-properties' internals, buffer positions), so the
;; oracle compares this projection instead — the honest intersection of
;; what Orgsidian's Story-2.3 semantic layer and org-element both model:
;;
;;   per headline, in document order, nested:
;;     level      int            number of stars
;;     todo       string|null    recognized TODO keyword
;;     title      string         org-element `:raw-value' (stars, keyword,
;;                               and trailing tags stripped; everything
;;                               else verbatim)
;;     tags       string array   trailing tags, in order, no colons
;;     scheduled  string|null    planning timestamp `:raw-value'
;;     deadline   string|null    planning timestamp `:raw-value'
;;     closed     string|null    planning timestamp `:raw-value'
;;     children   array          recursive
;;
;; Schema/regeneration/triage docs: docs/parser/l2-oracle.md.
;; Rust mirror: crates/orgsidian-parser/tests/l2_canonical.rs.
;;
;; Usage (must run clean with no user init and no network):
;;   emacs -Q --batch -l scripts/l2-oracle/projection.el \
;;     --eval '(l2-project-file "tests/fixtures/vault-corpus/extracted/NNNN_x.org")'
;;
;; Requires Emacs >= 27 (builtin `json-serialize'). Determinism notes:
;; key order is fixed by plist construction below; `:null' is the JSON
;; null; strings are de-propertized via `substring-no-properties'; arrays
;; are vectors so empty arrays serialize as [] (never null).

(require 'org)
(require 'org-element)

(defun l2--string (s)
  "S de-propertized, or `:null' when S is nil."
  (if s (substring-no-properties s) :null))

(defun l2--timestamp-raw (ts)
  "The `:raw-value' of timestamp node TS, or `:null' when TS is nil."
  (if ts (l2--string (org-element-property :raw-value ts)) :null))

(defun l2--project-children (node)
  "Vector of schema-v1 projections of NODE's direct child headlines."
  (vconcat
   (delq nil
         (mapcar (lambda (el)
                   (when (eq (org-element-type el) 'headline)
                     (l2--project-headline el)))
                 (org-element-contents node)))))

(defun l2--project-headline (hl)
  "Schema-v1 plist for headline node HL (key order = schema order)."
  (list :level (org-element-property :level hl)
        :todo (l2--string (org-element-property :todo-keyword hl))
        :title (let ((raw (org-element-property :raw-value hl)))
                 (substring-no-properties (or raw "")))
        :tags (vconcat (mapcar #'substring-no-properties
                               (org-element-property :tags hl)))
        :scheduled (l2--timestamp-raw (org-element-property :scheduled hl))
        :deadline (l2--timestamp-raw (org-element-property :deadline hl))
        :closed (l2--timestamp-raw (org-element-property :closed hl))
        :children (l2--project-children hl)))

(defun l2-project-file (file)
  "Print the schema-v1 headline projection of FILE as JSON on stdout.

The output is the bare headlines ARRAY (compact JSON, one trailing
newline). The committed canonical files wrap it with source/schema/
deftest metadata — see scripts/l2-oracle/generate-canonical.sh."
  (with-temp-buffer
    (insert-file-contents file)
    ;; Full org-mode startup: in-buffer #+TODO:/#+SEQ_TODO: directives must
    ;; be honored exactly as org-element does interactively.
    (org-mode)
    (princ (json-serialize (l2--project-children (org-element-parse-buffer))
                           :null-object :null))
    (princ "\n")))

;;; projection.el ends here
