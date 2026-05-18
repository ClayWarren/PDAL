#pragma once

#include <pdal/PointTable.hpp>
#include <pdal/PointView.hpp>
#include <pdal/SpatialReference.hpp>
#include <pdal/pdal_types.hpp>
#include <pdal_capi.h>

#include <memory>
#include <string>
#include <utility>
#include <vector>

namespace pdal
{

class RustPipeline
{
public:
    RustPipeline() : m_pipeline(pdal_pipeline_create()) {}

    ~RustPipeline()
    {
        if (m_pipeline)
            pdal_pipeline_destroy(m_pipeline);
    }

    // Non-copyable, non-movable (owns raw C handle)
    RustPipeline(const RustPipeline&) = delete;
    RustPipeline& operator=(const RustPipeline&) = delete;

    /// Add a stage to the pipeline. The stage is consumed and must not be
    /// used after this call. Returns the stage index.
    int64_t addStage(pdal_stage_t* stage)
    {
        int64_t idx = pdal_pipeline_add_stage(m_pipeline, stage);
        if (idx < 0)
            throw pdal_error("Failed to add stage to Rust pipeline.");
        return idx;
    }

    /// Add a reader to the pipeline. The reader is consumed and must not be
    /// used after this call. Returns the stage index.
    int64_t addReader(pdal_reader_t* reader)
    {
        int64_t idx = pdal_pipeline_add_reader(m_pipeline, reader);
        if (idx < 0)
            throw pdal_error("Failed to add reader to Rust pipeline.");
        return idx;
    }

    /// Add a writer to the pipeline. The writer is consumed and must not be
    /// used after this call. Returns the stage index.
    int64_t addWriter(pdal_writer_t* writer)
    {
        int64_t idx = pdal_pipeline_add_writer(m_pipeline, writer);
        if (idx < 0)
            throw pdal_error("Failed to add writer to Rust pipeline.");
        return idx;
    }

    /// Add a stage with a tag for later reference.
    int64_t addStageTagged(pdal_stage_t* stage, const std::string& tag)
    {
        int64_t idx =
            pdal_pipeline_add_stage_tagged(m_pipeline, stage, tag.c_str());
        if (idx < 0)
        {
            const char* err = pdal_last_error();
            throw pdal_error(err && err[0] ? err
                                           : "Failed to add tagged stage.");
        }
        return idx;
    }

    /// Declare that `target` depends on `input`.
    void addDependency(uint64_t target, uint64_t input)
    {
        int64_t result =
            pdal_pipeline_add_dependency(m_pipeline, target, input);
        if (result < 0)
        {
            const char* err = pdal_last_error();
            throw pdal_error(err && err[0] ? err
                                           : "Failed to add pipeline "
                                             "dependency.");
        }
    }

    /// Execute the pipeline with an input view. Returns the output view.
    /// Note: the input view is consumed by the pipeline and must not be
    /// destroyed by the caller.
    /// If `inView` is null, the pipeline is executed with no input views
    /// (reader-driven pipeline).
    PointViewPtr execute(PointViewPtr inView)
    {
        pdal_point_view_t* rustIn = nullptr;
        if (inView)
            rustIn = toRust(*inView);

        pdal_point_view_t* rustOut = pdal_pipeline_execute(m_pipeline, rustIn);
        // rustIn is consumed by pdal_pipeline_execute, do not destroy

        if (!rustOut)
        {
            const char* err = pdal_last_error();
            if (err && err[0])
                throw pdal_error(err);
            return PointViewPtr();
        }

        PointViewPtr outView = fromRust(rustOut, inView);
        pdal_point_view_destroy(rustOut);
        return outView;
    }

    /// Execute and return the output point count.
    /// Note: the input view is consumed by the pipeline.
    uint64_t executeCount(PointViewPtr inView)
    {
        pdal_point_view_t* rustIn = nullptr;
        if (inView)
            rustIn = toRust(*inView);
        int64_t count = pdal_pipeline_execute_count(m_pipeline, rustIn);
        // rustIn is consumed by pdal_pipeline_execute_count, do not destroy

        if (count < 0)
        {
            const char* err = pdal_last_error();
            throw pdal_error(err && err[0] ? err
                                           : "Rust pipeline execution failed.");
        }
        return static_cast<uint64_t>(count);
    }

    /// Number of stages in the pipeline.
    uint64_t stageCount() const
    {
        return pdal_pipeline_stage_count(m_pipeline);
    }

    /// Find a stage index by tag. Returns -1 if not found.
    int64_t findByTag(const std::string& tag) const
    {
        return pdal_pipeline_find_by_tag(m_pipeline, tag.c_str());
    }

    /// Get pipeline metadata as a C++ MetadataNode.
    MetadataNode metadata() const
    {
        pdal_metadata_node_t* rustMeta = pdal_pipeline_metadata(m_pipeline);
        if (!rustMeta)
            return MetadataNode("pipeline");

        MetadataNode result = fromRustMetadata(rustMeta);
        pdal_metadata_node_destroy(rustMeta);
        return result;
    }

private:
    pdal_pipeline_t* m_pipeline;
    std::vector<std::unique_ptr<PointTable>> m_ownedTables;

    // PointView conversion helpers (mirrors RustViewConverter pattern)

    static int typeId(Dimension::Type type)
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

    static std::string takeString(char* value)
    {
        if (!value)
            return std::string();
        std::string result(value);
        pdal_string_free(value);
        return result;
    }

    static pdal_point_view_t* toRust(PointView& inView)
    {
        pdal_point_layout_t* layout = pdal_point_layout_create();
        for (auto dim : inView.layout()->dims())
        {
            pdal_point_layout_register_dim(
                layout, inView.layout()->dimName(dim).c_str(),
                typeId(inView.layout()->dimType(dim)));
        }
        pdal_point_view_t* rustInView = pdal_point_view_create(layout);

        // Set spatial reference
        pdal_spatial_reference_t* rustSrs =
            pdal_spatial_reference_create_with_epoch(
                inView.spatialReference().getWKT().c_str(),
                inView.spatialReference().getEpoch());
        pdal_point_view_set_spatial_reference(rustInView, rustSrs);
        pdal_spatial_reference_destroy(rustSrs);

        for (PointId idx = 0; idx < inView.size(); ++idx)
        {
            pdal_point_view_add_point(rustInView);
            for (auto dim : inView.layout()->dims())
            {
                double v = inView.getFieldAs<double>(dim, idx);
                pdal_point_view_set_f64(
                    rustInView, idx, inView.layout()->dimName(dim).c_str(), v);
            }
        }
        return rustInView;
    }

    static void verifyRustDims(pdal_point_view_t* rustView,
                               PointLayoutPtr layout)
    {
        uint64_t dimCount = pdal_point_view_dim_count(rustView);
        for (uint64_t idx = 0; idx < dimCount; ++idx)
        {
            std::string name =
                takeString(pdal_point_view_dim_name(rustView, idx));
            if (name.empty())
                continue;
            if (layout->findDim(name) == Dimension::Id::Unknown)
                throw pdal_error("Rust pipeline returned unregistered "
                                 "dimension '" +
                                 name + "'.");
        }
    }

    static SpatialReference spatialReference(pdal_point_view_t* rustView)
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

    PointViewPtr fromRust(pdal_point_view_t* rustOutView, PointViewPtr baseView)
    {
        uint64_t outLen = pdal_point_view_length(rustOutView);
        if (outLen == 0)
            return PointViewPtr();

        if (baseView)
        {
            PointViewPtr outView(new PointView(baseView->table(),
                                               spatialReference(rustOutView)));
            verifyRustDims(rustOutView, outView->layout());
            for (PointId idx = 0; idx < outLen; ++idx)
            {
                PointId sourceIdx =
                    pdal_point_view_source_index(rustOutView, idx);
                outView->appendPoint(*baseView, sourceIdx);
                PointId outIdx = outView->size() - 1;
                for (auto dim : outView->layout()->dims())
                {
                    double v = pdal_point_view_get_f64(
                        rustOutView, idx,
                        outView->layout()->dimName(dim).c_str());
                    outView->setField(dim, outIdx, v);
                }
            }
            return outView;
        }
        else
        {
            m_ownedTables.emplace_back(new PointTable());
            PointTable& table = *m_ownedTables.back();
            PointLayoutPtr layout = table.layout();

            uint64_t dimCount = pdal_point_view_dim_count(rustOutView);
            std::vector<std::pair<std::string, Dimension::Id>> dims;
            for (uint64_t i = 0; i < dimCount; ++i)
            {
                std::string name =
                    takeString(pdal_point_view_dim_name(rustOutView, i));
                if (!name.empty())
                {
                    Dimension::Id id = layout->registerOrAssignDim(
                        name, Dimension::Type::Double);
                    if (id != Dimension::Id::Unknown)
                        dims.push_back(std::make_pair(name, id));
                }
            }

            table.finalize();

            PointViewPtr outView(
                new PointView(table, spatialReference(rustOutView)));
            for (PointId idx = 0; idx < outLen; ++idx)
            {
                outView->point(idx);
                for (auto dim : dims)
                {
                    double v = pdal_point_view_get_f64(rustOutView, idx,
                                                       dim.first.c_str());
                    outView->setField(dim.second, idx, v);
                }
            }
            return outView;
        }
    }

    static MetadataNode fromRustMetadata(pdal_metadata_node_t* rustNode)
    {
        MetadataNode node(takeString(pdal_metadata_node_name(rustNode)));

        uint8_t kind = pdal_metadata_node_value_kind(rustNode);
        if (kind != 0xFF)
        {
            switch (kind)
            {
            case 0: // String
                node.add("value",
                         takeString(pdal_metadata_node_value(rustNode)));
                break;
            case 1: // I64
                node.add("value", pdal_metadata_node_value_i64(rustNode));
                break;
            case 2: // U64
                node.add("value", pdal_metadata_node_value_u64(rustNode));
                break;
            case 3: // F64
                node.add("value", pdal_metadata_node_value_f64(rustNode));
                break;
            case 4: // Bool
                node.add("value", pdal_metadata_node_value_bool(rustNode));
                break;
            }
        }

        uint64_t childCount = pdal_metadata_node_child_count(rustNode);
        for (uint64_t i = 0; i < childCount; ++i)
        {
            pdal_metadata_node_t* rustChild =
                pdal_metadata_node_child(rustNode, i);
            node.add(fromRustMetadata(rustChild));
            pdal_metadata_node_destroy(rustChild);
        }

        return node;
    }
};

} // namespace pdal
