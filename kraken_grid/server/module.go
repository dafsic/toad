package server

import (
	"context"
	"net"

	"github.com/dafsic/toad/kraken_grid/api"
	pb "github.com/dafsic/toad/proto_go/kraken_grid"
	"github.com/urfave/cli/v2"
	"go.uber.org/fx"
	"go.uber.org/zap"
	"google.golang.org/grpc"
)

const ModuleName = "server"

type Module struct{}

func (m *Module) Configure(app *cli.App) {
	app.Flags = append(app.Flags,
		&cli.StringFlag{
			Name:    "grpc_addr",
			EnvVars: []string{"GRPC_ADDR"},
			Value:   ":50051",
			Usage:   "gRPC server address",
		},
	)
}

func (m *Module) Install(ctx *cli.Context) fx.Option {
	return fx.Module(ModuleName,
		fx.Provide(
			NewServer,
		),
		fx.Invoke(func(lc fx.Lifecycle, logger *zap.Logger, api api.API, srv Server) {
			grpcServer := grpc.NewServer()
			grpcImpl := &GRPCServer{
				Server:                               srv,
				UnimplementedKrakenGridServiceServer: pb.UnimplementedKrakenGridServiceServer{},
			}
			pb.RegisterKrakenGridServiceServer(grpcServer, grpcImpl)

			addr := ctx.String("grpc_addr")

			lc.Append(fx.Hook{
				OnStart: func(ctx context.Context) error {
					lis, err := net.Listen("tcp", addr)
					if err != nil {
						logger.Error("Failed to listen", zap.String("address", addr), zap.Error(err))
						return err
					}

					logger.Info("Starting gRPC server", zap.String("address", addr))
					go func() {
						if err := grpcServer.Serve(lis); err != nil {
							logger.Error("Failed to serve gRPC", zap.Error(err))
						}
					}()
					return nil
				},
				OnStop: func(ctx context.Context) error {
					logger.Info("Stopping gRPC server")
					grpcServer.GracefulStop()
					return nil
				},
			})
		}),
	)
}

// GRPCServer implements the gRPC interface, extending our existing service implementation
type GRPCServer struct {
	pb.UnimplementedKrakenGridServiceServer
	Server Server
}

var _ pb.KrakenGridServiceServer = (*GRPCServer)(nil)

// PlaceOrder implements the gRPC interface, forwarding requests to our service implementation
func (s *GRPCServer) PlaceOrder(ctx context.Context, req *pb.PlaceOrderRequest) (*pb.Response, error) {
	return s.Server.PlaceOrder(ctx, req)
}

// Stop implements the gRPC interface, forwarding requests to our service implementation
func (s *GRPCServer) Stop(ctx context.Context, req *pb.StopRequest) (*pb.Response, error) {
	return s.Server.Stop(ctx, req)
}

// Run implements the gRPC interface, forwarding requests to our service implementation
func (s *GRPCServer) Run(ctx context.Context, req *pb.RunRequest) (*pb.Response, error) {
	return s.Server.Run(ctx, req)
}

// Status implements the gRPC interface, forwarding requests to our service implementation
func (s *GRPCServer) Status(ctx context.Context, req *pb.StatusRequest) (*pb.StatusResponse, error) {
	return s.Server.Status(ctx, req)
}

// SetBasePrice implements the gRPC interface, forwarding requests to our service implementation
func (s *GRPCServer) SetBasePrice(ctx context.Context, req *pb.SetBasePriceRequest) (*pb.Response, error) {
	return s.Server.SetBasePrice(ctx, req)
}
