
#include "NullWriter.hpp"

namespace pdal
{

static StaticPluginInfo const s_info{
    "writers.null",
    "Null writer.  Provides a sink for points in a pipeline.  "
    "It's the same as sending pipeline output to /dev/null.",
    "https://pdal.org/stages/writers.null.html"};

CREATE_STATIC_STAGE(NullWriter, s_info)

namespace
{

pdal_point_view_t* makeRustView(point_count_t count)
{
    pdal_point_layout_t* layout = pdal_point_layout_create();
    pdal_point_view_t* view = pdal_point_view_create(layout);
    for (point_count_t i = 0; i < count; ++i)
        pdal_point_view_add_point(view);
    return view;
}

void throwLastRustError(const std::string& fallback)
{
    const char* message = pdal_last_error();
    if (message && message[0])
        throw pdal_error(message);
    throw pdal_error(fallback);
}

} // namespace

NullWriter::NullWriter() {}

NullWriter::~NullWriter()
{
    if (m_rustWriter)
        pdal_writer_destroy(m_rustWriter);
}

std::string NullWriter::getName() const
{
    return s_info.name;
}

void NullWriter::ready(PointTableRef)
{
    if (m_rustWriter)
        pdal_writer_destroy(m_rustWriter);

    pdal_options_t* options = pdal_options_create();
    m_rustWriter = pdal_writer_create_null(options);
    pdal_options_destroy(options);

    if (!m_rustWriter)
        throwLastRustError("Failed to create Rust null writer.");
}

void NullWriter::write(const PointViewPtr view)
{
    pdal_point_view_t* rustView = makeRustView(view->size());
    bool ok = pdal_writer_write_view(m_rustWriter, rustView);
    pdal_point_view_destroy(rustView);
    if (!ok)
        throwLastRustError("Rust null writer failed.");
}

bool NullWriter::processOne(PointRef&)
{
    pdal_point_view_t* rustView = makeRustView(1);
    bool ok = pdal_writer_write_view(m_rustWriter, rustView);
    pdal_point_view_destroy(rustView);
    if (!ok)
        throwLastRustError("Rust null writer failed.");
    return true;
}

void NullWriter::done(PointTableRef)
{
    pdal_writer_destroy(m_rustWriter);
    m_rustWriter = nullptr;
}

} // namespace pdal
