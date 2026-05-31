#pragma once

#include <pdal/PointView.hpp>
#include <pdal/SpatialReference.hpp>
#include <pdal/pdal_types.hpp>
#include <pdal/private/Raster.hpp>
#include <pdal_capi.h>

#include <string>
#include <vector>

namespace pdal
{
namespace rust_view_converter
{

inline void throwLastError(const std::string& fallback)
{
    const char* message = pdal_last_error();
    if (message && message[0])
        throw pdal_error(message);
    throw pdal_error(fallback);
}

inline bool hasLastError()
{
    const char* message = pdal_last_error();
    return message && message[0];
}

inline std::string takeString(char* value)
{
    if (!value)
        return std::string();
    std::string result(value);
    pdal_string_free(value);
    return result;
}

inline void setSpatialReference(pdal_point_view_t* rustView,
                                const SpatialReference& srs)
{
    pdal_spatial_reference_t* rustSrs =
        pdal_spatial_reference_create_with_epoch(srs.getWKT().c_str(),
                                                 srs.getEpoch());
    pdal_point_view_set_spatial_reference(rustView, rustSrs);
    pdal_spatial_reference_destroy(rustSrs);
}

inline SpatialReference spatialReference(pdal_point_view_t* rustView)
{
    pdal_spatial_reference_t* rustSrs =
        pdal_point_view_spatial_reference(rustView);
    if (!rustSrs)
        return SpatialReference();

    SpatialReference srs(takeString(pdal_spatial_reference_text(rustSrs)));
    srs.setEpoch(pdal_spatial_reference_epoch(rustSrs));
    pdal_spatial_reference_destroy(rustSrs);
    return srs;
}

inline pdal_raster_limits_t toRustLimits(const RasterLimits& limits)
{
    pdal_raster_limits_t rustLimits;
    rustLimits.x_origin = limits.xOrigin;
    rustLimits.y_origin = limits.yOrigin;
    rustLimits.width = limits.width;
    rustLimits.height = limits.height;
    rustLimits.edge_length = limits.edgeLength;
    return rustLimits;
}

inline RasterLimits fromRustLimits(const pdal_raster_limits_t& limits)
{
    return RasterLimits(limits.x_origin, limits.y_origin,
                        static_cast<int>(limits.width),
                        static_cast<int>(limits.height), limits.edge_length);
}

inline int typeId(Dimension::Type type)
{
    using Dimension::Type;
    switch (type)
    {
    case Type::Unsigned8:
        return 0;
    case Type::Unsigned16:
        return 1;
    case Type::Unsigned32:
        return 2;
    case Type::Unsigned64:
        return 3;
    case Type::Signed8:
        return 4;
    case Type::Signed16:
        return 5;
    case Type::Signed32:
        return 6;
    case Type::Signed64:
        return 7;
    case Type::Float:
        return 8;
    case Type::Double:
    case Type::None:
        return 9;
    }
    return 9;
}

inline void verifyRustDims(pdal_point_view_t* rustView, PointLayoutPtr layout)
{
    uint64_t dimCount = pdal_point_view_dim_count(rustView);
    for (uint64_t idx = 0; idx < dimCount; ++idx)
    {
        std::string name = takeString(pdal_point_view_dim_name(rustView, idx));
        if (name.empty())
            continue;
        if (layout->findDim(name) == Dimension::Id::Unknown)
            throw pdal_error("Rust stage returned unregistered dimension '" +
                             name + "'.");
    }
}

inline pdal_point_layout_t* toRustLayout(PointLayoutPtr layout)
{
    pdal_point_layout_t* rustLayout = pdal_point_layout_create();
    for (auto dim : layout->dims())
    {
        pdal_point_layout_register_dim(rustLayout, layout->dimName(dim).c_str(),
                                       typeId(layout->dimType(dim)));
    }
    return rustLayout;
}

inline pdal_point_view_t* toRust(PointView& inView)
{
    pdal_point_layout_t* layout = toRustLayout(inView.layout());
    pdal_point_view_t* rust_in_view = pdal_point_view_create(layout);
    setSpatialReference(rust_in_view, inView.spatialReference());
    for (PointId idx = 0; idx < inView.size(); ++idx)
    {
        pdal_point_view_add_point(rust_in_view);
        for (auto dim : inView.layout()->dims())
        {
            double v = inView.getFieldAs<double>(dim, idx);
            pdal_point_view_set_f64(rust_in_view, idx,
                                    inView.layout()->dimName(dim).c_str(), v);
        }
    }
    if (TriangularMesh* mesh = inView.mesh())
    {
        for (const Triangle& triangle : *mesh)
        {
            pdal_point_view_add_mesh_triangle(rust_in_view, triangle.m_a,
                                              triangle.m_b, triangle.m_c);
        }
    }
    if (Rasterd* raster = inView.raster())
    {
        pdal_raster_limits_t limits = toRustLimits(raster->limits());
        if (pdal_point_view_create_raster(rust_in_view, raster->name().c_str(),
                                          &limits, raster->initializer()))
        {
            for (int y = 0; y < raster->height(); ++y)
            {
                for (int x = 0; x < raster->width(); ++x)
                {
                    pdal_point_view_set_raster_cell(rust_in_view,
                                                    raster->name().c_str(), x,
                                                    y, raster->at(x, y));
                }
            }
        }
    }
    // Ownership of layout is transferred to rust_in_view. Do not destroy layout
    // here.
    return rust_in_view;
}

inline pdal_point_view_t* toRust(PointViewPtr inView)
{
    return toRust(*inView);
}

// Convert a single streaming PointRef into a one-point Rust view, so a
// data-dependent filter can be streamed through pdal_stage_process_one_at.
inline pdal_point_view_t* toRustPoint(PointRef& point, PointLayoutPtr layout)
{
    pdal_point_layout_t* rustLayout = toRustLayout(layout);
    pdal_point_view_t* rustView = pdal_point_view_create(rustLayout);
    pdal_point_view_add_point(rustView);
    for (auto dim : layout->dims())
    {
        double v = point.getFieldAs<double>(dim);
        pdal_point_view_set_f64(rustView, 0, layout->dimName(dim).c_str(), v);
    }
    return rustView;
}

inline void fromRustPoint(pdal_point_view_t* rust_out_view, uint64_t rust_idx,
                          PointRef& outPoint)
{
    if (rust_out_view)
    {
        uint64_t dimCount = pdal_point_view_dim_count(rust_out_view);
        for (uint64_t i = 0; i < dimCount; ++i)
        {
            std::string dimName =
                takeString(pdal_point_view_dim_name(rust_out_view, i));
            Dimension::Id id = outPoint.layout()->findDim(dimName);
            if (id != Dimension::Id::Unknown)
            {
                double v = pdal_point_view_get_f64(rust_out_view, rust_idx,
                                                   dimName.c_str());
                outPoint.setField(id, v);
            }
        }
    }
}

inline void copyMeshFromRust(pdal_point_view_t* rustView, PointView& outView)
{
    uint64_t triangleCount = pdal_point_view_mesh_triangle_count(rustView);
    if (!triangleCount)
        return;

    TriangularMesh* mesh = outView.mesh();
    if (!mesh)
        mesh = outView.createMesh("");
    if (!mesh)
        return;

    for (uint64_t idx = 0; idx < triangleCount; ++idx)
    {
        uint64_t a = 0;
        uint64_t b = 0;
        uint64_t c = 0;
        if (pdal_point_view_mesh_triangle(rustView, idx, &a, &b, &c))
            mesh->add(a, b, c);
    }
}

inline void copyRastersFromRust(pdal_point_view_t* rustView, PointView& outView)
{
    uint64_t rasterCount = pdal_point_view_raster_count(rustView);
    for (uint64_t idx = 0; idx < rasterCount; ++idx)
    {
        std::string name =
            takeString(pdal_point_view_raster_name(rustView, idx));
        pdal_raster_limits_t rustLimits;
        if (!pdal_point_view_raster_limits(rustView, name.c_str(), &rustLimits))
            continue;

        Rasterd* raster = outView.raster(name);
        if (!raster)
        {
            raster = outView.createRaster(
                name, fromRustLimits(rustLimits),
                pdal_point_view_raster_initializer(rustView, name.c_str()));
        }
        if (!raster)
            continue;

        for (int y = 0; y < raster->height(); ++y)
        {
            for (int x = 0; x < raster->width(); ++x)
            {
                double value = 0;
                if (pdal_point_view_raster_cell(rustView, name.c_str(), x, y,
                                                &value))
                {
                    raster->at(x, y) = value;
                }
            }
        }
    }
}

inline void fromRust(pdal_point_view_t* rust_out_view, PointView& outView)
{
    if (rust_out_view)
    {
        verifyRustDims(rust_out_view, outView.layout());
        uint64_t out_len = pdal_point_view_length(rust_out_view);
        for (PointId idx = 0; idx < out_len; ++idx)
        {
            PointId out_idx = idx;
            if (out_idx >= outView.size())
            {
                outView.point(out_idx);
            }
            for (auto dim : outView.layout()->dims())
            {
                double v = pdal_point_view_get_f64(
                    rust_out_view, idx, outView.layout()->dimName(dim).c_str());
                outView.setField(dim, out_idx, v);
            }
        }
        copyMeshFromRust(rust_out_view, outView);
        copyRastersFromRust(rust_out_view, outView);
    }
}

inline void fromRust(pdal_point_view_t* rust_out_view, PointViewPtr baseView,
                     PointView& outView)
{
    if (rust_out_view)
    {
        verifyRustDims(rust_out_view, outView.layout());
        uint64_t out_len = pdal_point_view_length(rust_out_view);
        for (PointId idx = 0; idx < out_len; ++idx)
        {
            PointId source_idx =
                pdal_point_view_source_index(rust_out_view, idx);
            outView.appendPoint(*baseView, source_idx);
            PointId out_idx = outView.size() - 1;
            for (auto dim : outView.layout()->dims())
            {
                double v = pdal_point_view_get_f64(
                    rust_out_view, idx, outView.layout()->dimName(dim).c_str());
                outView.setField(dim, out_idx, v);
            }
        }
        copyMeshFromRust(rust_out_view, outView);
        copyRastersFromRust(rust_out_view, outView);
    }
}

inline PointViewPtr fromRust(pdal_point_view_t* rust_out_view,
                             PointViewPtr baseView)
{
    PointViewPtr outView(
        new PointView(baseView->table(), spatialReference(rust_out_view)));
    fromRust(rust_out_view, baseView, *outView);
    return outView;
}

inline PointViewPtr runSingle(pdal_stage_t* stage, PointViewPtr inView)
{
    pdal_point_view_t* rustIn = toRust(inView);
    pdal_point_view_t* rustOut = pdal_stage_run(stage, rustIn);
    pdal_point_view_destroy(rustIn);

    if (!rustOut)
        throwLastError("Rust stage failed.");

    PointViewPtr outView = fromRust(rustOut, inView);
    pdal_point_view_destroy(rustOut);
    return outView;
}

inline void runInPlace(pdal_stage_t* stage, PointView& view)
{
    pdal_point_view_t* rustIn = toRust(view);
    pdal_point_view_t* rustOut = pdal_stage_run(stage, rustIn);
    pdal_point_view_destroy(rustIn);

    if (!rustOut)
        throwLastError("Rust stage failed.");

    fromRust(rustOut, view);
    pdal_point_view_destroy(rustOut);
}

inline void runInPlaceWithReference(pdal_stage_t* stage, PointView& view,
                                    PointView& referenceView)
{
    pdal_point_view_t* rustIn = toRust(view);
    pdal_point_view_t* rustReference = toRust(referenceView);
    pdal_point_view_t* rustOut =
        pdal_stage_run_with_reference(stage, rustIn, rustReference);
    pdal_point_view_destroy(rustIn);
    pdal_point_view_destroy(rustReference);

    if (!rustOut)
        throwLastError("Rust stage failed.");

    fromRust(rustOut, view);
    pdal_point_view_destroy(rustOut);
}

inline void runInto(pdal_stage_t* stage, PointViewPtr inView,
                    PointView& outView)
{
    pdal_point_view_t* rustIn = toRust(inView);
    pdal_point_view_t* rustOut = pdal_stage_run(stage, rustIn);
    pdal_point_view_destroy(rustIn);

    if (!rustOut)
        throwLastError("Rust stage failed.");

    fromRust(rustOut, outView);
    pdal_point_view_destroy(rustOut);
}

inline PointViewSet runMulti(pdal_stage_t* stage, PointViewPtr inView,
                             uint64_t maxOutputs)
{
    pdal_point_view_t* rustIn = toRust(inView);
    std::vector<pdal_point_view_t*> rustOutputs(maxOutputs, nullptr);
    uint64_t count =
        pdal_stage_run_multi(stage, rustIn, rustOutputs.data(), maxOutputs);
    pdal_point_view_destroy(rustIn);

    if (count == 0)
    {
        for (pdal_point_view_t* output : rustOutputs)
            pdal_point_view_destroy(output);
        if (hasLastError())
            throwLastError("Rust stage failed.");
    }

    PointViewSet viewSet;
    for (uint64_t i = 0; i < count; ++i)
    {
        if (rustOutputs[i])
        {
            viewSet.insert(fromRust(rustOutputs[i], inView));
            pdal_point_view_destroy(rustOutputs[i]);
        }
    }
    return viewSet;
}

} // namespace rust_view_converter
} // namespace pdal
