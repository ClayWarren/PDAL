#pragma once

#include <pdal/PointView.hpp>
#include <pdal/SpatialReference.hpp>
#include <pdal/pdal_types.hpp>
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

inline pdal_point_view_t* toRust(PointView& inView)
{
    pdal_point_layout_t* layout = pdal_point_layout_create();
    for (auto dim : inView.layout()->dims())
    {
        pdal_point_layout_register_dim(
            layout, inView.layout()->dimName(dim).c_str(), 9);
    }
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
    // Ownership of layout is transferred to rust_in_view. Do not destroy layout
    // here.
    return rust_in_view;
}

inline pdal_point_view_t* toRust(PointViewPtr inView)
{
    return toRust(*inView);
}

inline void fromRust(pdal_point_view_t* rust_out_view, PointView& outView)
{
    if (rust_out_view)
    {
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
    }
}

inline void fromRust(pdal_point_view_t* rust_out_view, PointViewPtr baseView,
                     PointView& outView)
{
    if (rust_out_view)
    {
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
