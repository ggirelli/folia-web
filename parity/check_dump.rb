# frozen_string_literal: true

# Checks a folia_dump.rb dump against the build it describes:
#
# - the manifest accounts for every file: each written row names a file that
#   exists, and each file is named by exactly one written row;
# - every last_modified_at is reproducible: none fell back to a file's mtime,
#   which depends on the checkout rather than on the commit.
#
#   ruby check_dump.rb DUMP_DIR SITE_DIR

require "json"

# Prints up to 20 paths under a heading, and returns whether there were none.
def report(label, paths)
  return true if paths.empty?

  warn "#{paths.size} #{label}:"
  paths.first(20).each { |p| warn "  #{p}" }
  false
end

def check_manifest(dump, site)
  rows = File.readlines(File.join(dump, "manifest.jsonl"), chomp: true).map { |line| JSON.parse(line) }
  written = rows.select { |r| r["written"] }.map { |r| r["output"] }
  files = Dir.glob("**/*", File::FNM_DOTMATCH, base: site).select { |f| File.file?(File.join(site, f)) }

  puts "Manifest: #{rows.size} rows, #{written.size} written, #{rows.size - written.size} not written; " \
       "site: #{files.size} files."
  ok = [
    report("written twice", written.tally.select { |_, n| n > 1 }.keys.sort),
    report("listed but not built", (written - files).uniq.sort),
    report("built but not listed", (files - written).sort)
  ].all?
  puts "Every built file is accounted for." if ok
  ok
end

def check_dates(dump)
  metas = Dir.glob("meta/**/*.json", base: dump).sort
  origins = metas.to_h { |m| [m, JSON.parse(File.read(File.join(dump, m)))["last_modified_at_from"]] }
  counts = origins.values.tally.sort_by { |origin, _| origin.to_s }.map { |origin, n| "#{origin || "none"} #{n}" }

  puts "Dates: #{counts.join(", ")}."
  ok = report("dates from file mtimes (commit the files, or leave them out)",
    origins.select { |_, origin| origin == "mtime" }.keys)
  puts "Every date is reproducible." if ok
  ok
end

dump, site = ARGV
abort "usage: ruby #{$PROGRAM_NAME} DUMP_DIR SITE_DIR" unless dump && site

exit 1 unless [check_manifest(dump, site), check_dates(dump)].all?
