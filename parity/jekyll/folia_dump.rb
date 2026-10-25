# frozen_string_literal: true

# Folia baseline dump (plan v2, §10 spike 0).
#
# A passive Jekyll plugin that records Jekyll's own intermediate results, so
# Folia can be compared against them without reverse-engineering page HTML.
# It is loaded from outside the site:
#
#   FOLIA_DUMP=<dir> bundle exec jekyll build \
#     --plugins <site>/_plugins,<folia>/parity/jekyll ...
#
# Without FOLIA_DUMP it registers no hooks. With it, the hooks only read site
# state, and every file is written below <dir>, never into the site source or
# destination.
#
# Layout of <dir>:
#   manifest.jsonl          one row per page, document and static file
#   content/<source>.html   body HTML after Liquid, generators and conversion,
#                           before layouts; pages that share a source (such
#                           as pagination) go to content/_by_output/<output>.html
#   excerpt/<source>.html   rendered excerpt, for documents that have one
#   meta/<source>.json      title, dates, categories, tags

require "fileutils"
require "json"
require "open3"
require "pathname"
require "safe_yaml"

module FoliaDump
  DIR = ENV.fetch("FOLIA_DUMP", "")

  module_function

  def enabled?
    !DIR.empty?
  end

  # --- Hooks ----------------------------------------------------------------

  # Body HTML as it stands between conversion and layouts. Later stages
  # replace item.content, so it is copied now and written after the build.
  def capture(item)
    captured[item] = item.content.to_s.dup if source_file?(item)
  end

  def dump(site)
    all = items(site)
    shared = shared_sources(site, all)
    rows = all.map { |item| write_item(site, item, shared) }
    rows.sort_by! { |r| [r["output"], r["kind"], r["source"].to_s, r["url"].to_s] }
    write_file("manifest.jsonl", rows.map { |r| "#{JSON.generate(r)}\n" }.join)
    Jekyll.logger.info "Folia dump:", "#{rows.size} manifest rows, #{captured.size} content dumps"
  end

  # --- Per-item dumps -------------------------------------------------------

  def write_item(site, item, shared)
    src = source(site, item)
    unique = source_file?(item) && !shared.include?(src)
    {
      "kind" => kind(item),
      "collection" => collection(item),
      "class" => item.class.name,
      "source" => src,
      "url" => item.url,
      "output" => output(site, item),
      "written" => written?(item),
      "content" => write_content(site, item, src, unique),
      "excerpt" => write_excerpt(item, src),
      "meta" => (write_file("meta/#{src}.json", pretty_json(meta(item))) if unique)
    }
  end

  def write_content(site, item, src, unique)
    return nil unless captured.key?(item)

    rel = unique ? "content/#{src}.html" : "content/_by_output/#{output(site, item)}.html"
    write_file(rel, captured[item])
  end

  def write_excerpt(item, src)
    return nil unless item.is_a?(Jekyll::Document) && written?(item)

    excerpt = item.data["excerpt"]
    return nil unless excerpt.is_a?(Jekyll::Excerpt)

    write_file("excerpt/#{src}.html", excerpt.output.to_s)
  end

  def meta(item)
    data = item.data
    date = item.is_a?(Jekyll::Document) ? item.date : data["date"]
    modified = data["last_modified_at"]
    {
      "title" => data["title"],
      "date" => iso8601(date),
      "last_modified_at" => iso8601(modified),
      "last_modified_at_from" => modified_origin(item, modified),
      "last_modified_at_front_matter" => front_matter_value(item, "last_modified_at"),
      "categories" => data["categories"],
      "tags" => data["tags"]
    }
  end

  # --- Site items -----------------------------------------------------------

  def items(site)
    site.pages + site.static_files + site.collections.values.flat_map(&:docs)
  end

  def shared_sources(site, items)
    items.select { |i| source_file?(i) }.map { |i| source(site, i) }.tally.select { |_, n| n > 1 }.keys
  end

  # True for pages and documents read from a file in the site source. Page
  # subclasses are generators (feeds, archives, redirect stubs, the garden
  # graph, …): some name a path where a stale file may sit, but never read it.
  def source_file?(item)
    return false unless item.is_a?(Jekyll::Document) || item.instance_of?(Jekyll::Page)

    path = absolute_path(item)
    File.file?(path) && path.start_with?(item.site.source + File::SEPARATOR)
  end

  # Documents and static files have absolute paths; pages, relative ones.
  def absolute_path(item)
    File.expand_path(item.path, item.is_a?(Jekyll::StaticFile) ? nil : item.site.source)
  end

  def source(site, item)
    return nil unless item.is_a?(Jekyll::StaticFile) || source_file?(item)

    relative(absolute_path(item), site.source)
  end

  def output(site, item)
    relative(item.destination(site.dest), site.dest)
  end

  def kind(item)
    case item
    when Jekyll::Document then "document"
    when Jekyll::StaticFile then "static"
    else "page"
    end
  end

  def collection(item)
    item.collection&.label if item.respond_to?(:collection)
  end

  def written?(item)
    item.respond_to?(:write?) ? item.write? : true
  end

  # --- Values ---------------------------------------------------------------

  def iso8601(value)
    case value
    when nil then nil
    when Time, Date then value.iso8601
    when Jekyll::LastModifiedAt::Determinator then plugin_time(value)&.iso8601
    else value.to_s
    end
  end

  # A Sass page shares its data with its source-map page, whose Determinator
  # points at a .css.map file that is never in the source. Jekyll never reads
  # that value, so it is dumped as unresolved instead of failing the build.
  def plugin_time(determinator)
    determinator.to_liquid
  rescue Errno::ENOENT
    nil
  end

  # jekyll-last-modified-at stores a Determinator, which resolves to the date
  # of the last commit touching the file, or to the file's mtime when no
  # commit does. Any other value came from front matter.
  def modified_origin(item, value)
    case value
    when nil then nil
    when Jekyll::LastModifiedAt::Determinator
      return "unresolved:#{value.page_path}" unless plugin_time(value)

      committed?(absolute_path(item)) ? "git" : "mtime"
    else "front_matter"
    end
  end

  # Mirrors the Determinator's own test: a file is dated by git when some
  # commit reachable from HEAD touches it. Running git from the file's
  # directory finds the repository that holds it, submodule or not.
  def committed?(path)
    out, status = Open3.capture2(
      "git", "-C", File.dirname(path), "log", "-n", "1", "--format=%ct", "--", File.basename(path)
    )
    status.success? && !out.strip.empty?
  end

  # For pages, jekyll-last-modified-at overwrites data["last_modified_at"]
  # after the front matter is read, so the authored value is read from the
  # file itself.
  def front_matter_value(item, key)
    match = Jekyll::Document::YAML_FRONT_MATTER_REGEXP.match(File.read(absolute_path(item), encoding: "UTF-8"))
    data = match && SafeYAML.load(match[1])
    data.is_a?(Hash) ? iso8601(data[key]) : nil
  end

  # --- Files ----------------------------------------------------------------

  def captured
    @captured ||= {}
  end

  def relative(path, base)
    Pathname.new(path).relative_path_from(Pathname.new(base)).to_s
  end

  def pretty_json(value)
    "#{JSON.pretty_generate(value)}\n"
  end

  # Writes below DIR and returns the relative path, for the manifest.
  def write_file(rel, body)
    path = File.join(DIR, rel)
    FileUtils.mkdir_p(File.dirname(path))
    File.binwrite(path, body)
    rel
  end
end

if FoliaDump.enabled?
  unless Dir.empty?(FoliaDump::DIR)
    raise Jekyll::Errors::FatalException, "FOLIA_DUMP=#{FoliaDump::DIR} must be an empty directory"
  end

  Jekyll::Hooks.register(%i[pages documents], :post_convert) { |item| FoliaDump.capture(item) }
  Jekyll::Hooks.register(:site, :post_write) { |site| FoliaDump.dump(site) }
end
