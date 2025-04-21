package postgres

import (
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"strconv"

	migrate "github.com/rubenv/sql-migrate"
	"go.uber.org/zap"
)

// MigrationDirection defines the direction of a migration
type MigrationDirection string

const (
	// MigrationDirectionUp applies the migrations
	MigrationDirectionUp MigrationDirection = "up"
	// MigrationDirectionDown reverts the migrations
	MigrationDirectionDown MigrationDirection = "down"
)

// MigrationResult contains information about applied migrations
type MigrationResult struct {
	Applied int
	Error   error
}

// MigrationOptions contains options for the migration process
type MigrationOptions struct {
	// Directory is the path to the directory containing migration files
	Directory string
	// TableName is the name of the table to use for migration state
	TableName string
	// SchemaName is the name of the schema to use for migration state
	SchemaName string
}

// DefaultMigrationOptions returns a MigrationOptions with default values
func DefaultMigrationOptions(directory string) MigrationOptions {
	return MigrationOptions{
		Directory:  directory,
		TableName:  "migrations",
		SchemaName: "public",
	}
}

// Migrator handles database migrations
type Migrator struct {
	db           *DatabaseImpl
	logger       *zap.Logger
	options      MigrationOptions
	versionRegex *regexp.Regexp
}

// NewMigrator creates a new migrator
func NewMigrator(db *DatabaseImpl, logger *zap.Logger, options MigrationOptions) *Migrator {
	return &Migrator{
		db:           db,
		logger:       logger,
		options:      options,
		versionRegex: regexp.MustCompile(`^V(\d+)__.*\.sql$`),
	}
}

// ApplyMigrations applies database migrations from the specified directory
func (impl *DatabaseImpl) ApplyMigrations(direction MigrationDirection, options MigrationOptions) (*MigrationResult, error) {
	impl.logger.Info("Applying migrations",
		zap.String("direction", string(direction)),
		zap.String("directory", options.Directory),
		zap.String("table", options.TableName),
		zap.String("schema", options.SchemaName),
	)

	// Check if the migration directory exists
	if _, err := os.Stat(options.Directory); os.IsNotExist(err) {
		return nil, fmt.Errorf("migration directory does not exist: %s", options.Directory)
	}

	// Configure the migration source
	source := &migrate.FileMigrationSource{
		Dir: options.Directory,
	}

	// Configure the migration table
	migrate.SetTable(options.TableName)
	migrate.SetSchema(options.SchemaName)

	// Determine the migration direction
	var migrationDirection migrate.MigrationDirection
	switch direction {
	case MigrationDirectionUp:
		migrationDirection = migrate.Up
	case MigrationDirectionDown:
		migrationDirection = migrate.Down
	default:
		return nil, fmt.Errorf("invalid migration direction: %s", direction)
	}

	// Apply the migrations
	applied, err := migrate.Exec(impl.db.DB, impl.driver, source, migrationDirection)
	if err != nil {
		return &MigrationResult{Applied: 0, Error: err}, err
	}

	impl.logger.Info("Migrations completed", zap.Int("applied", applied))
	return &MigrationResult{Applied: applied, Error: nil}, nil
}

// GetMigrationStatus returns the status of all migrations
func (impl *DatabaseImpl) GetMigrationStatus(options MigrationOptions) ([]*migrate.MigrationRecord, error) {
	impl.logger.Info("Getting migration status",
		zap.String("directory", options.Directory),
		zap.String("table", options.TableName),
		zap.String("schema", options.SchemaName),
	)

	// Check if the migration directory exists
	if _, err := os.Stat(options.Directory); os.IsNotExist(err) {
		return nil, fmt.Errorf("migration directory does not exist: %s", options.Directory)
	}

	// Configure the migration source
	source := &migrate.FileMigrationSource{
		Dir: options.Directory,
	}

	// Configure the migration table
	migrate.SetTable(options.TableName)
	migrate.SetSchema(options.SchemaName)

	// Get all migrations
	_, err := source.FindMigrations()
	if err != nil {
		return nil, err
	}

	// Get the applied migrations
	records, err := migrate.GetMigrationRecords(impl.db.DB, impl.driver)
	if err != nil {
		return nil, err
	}

	return records, nil
}

// FindMigrations returns all migration files from the specified directory
func (impl *DatabaseImpl) FindMigrations(directory string) ([]*migrate.Migration, error) {
	impl.logger.Info("Finding migrations", zap.String("directory", directory))

	// Check if the migration directory exists
	if _, err := os.Stat(directory); os.IsNotExist(err) {
		return nil, fmt.Errorf("migration directory does not exist: %s", directory)
	}

	// Configure the migration source
	source := &migrate.FileMigrationSource{
		Dir: directory,
	}

	// Find all migrations
	migrations, err := source.FindMigrations()
	if err != nil {
		return nil, err
	}

	return migrations, nil
}

// CreateMigration creates a new empty migration file
func (impl *DatabaseImpl) CreateMigration(name, directory string) (string, error) {
	impl.logger.Info("Creating migration", zap.String("name", name), zap.String("directory", directory))

	// Check if the migration directory exists, create it if it doesn't
	if _, err := os.Stat(directory); os.IsNotExist(err) {
		if err := os.MkdirAll(directory, 0755); err != nil {
			return "", err
		}
	}

	// Format the filename
	timestamp := impl.getTimestamp(directory)
	filename := fmt.Sprintf("V%s__%s.sql", timestamp, name)
	path := filepath.Join(directory, filename)

	// Create the file with the migration template
	content := `-- +migrate Up
SET LOCAL statement_timeout = '15s';

-- Write your Up migration here

-- +migrate Down
SET LOCAL statement_timeout = '15s';

-- Write your Down migration here
`

	// Write the file
	if err := os.WriteFile(path, []byte(content), 0644); err != nil {
		return "", err
	}

	return path, nil
}

// getTimestamp returns a timestamp for migration filenames
func (impl *DatabaseImpl) getTimestamp(directory string) string {
	// Use the same format as in your existing migration file (e.g., "001")
	return fmt.Sprintf("%03d", impl.getNextMigrationNumber(directory))
}

// getNextMigrationNumber calculates the next migration number by scanning the directory
func (impl *DatabaseImpl) getNextMigrationNumber(directory string) int {
	// Create a regex to extract version numbers from migration filenames
	re := regexp.MustCompile(`^V(\d+)__.*\.sql$`)

	// Read the directory
	files, err := os.ReadDir(directory)
	if err != nil {
		// If there's an error, just return 1
		return 1
	}

	highest := 0
	for _, file := range files {
		if file.IsDir() {
			continue
		}

		matches := re.FindStringSubmatch(file.Name())
		if len(matches) < 2 {
			continue
		}

		num, err := strconv.Atoi(matches[1])
		if err != nil {
			continue
		}

		if num > highest {
			highest = num
		}
	}

	// Return the next number
	return highest + 1
}

// ApplyMigrations applies all pending migrations
func (m *Migrator) ApplyMigrations() (*MigrationResult, error) {
	return m.db.ApplyMigrations(MigrationDirectionUp, m.options)
}

// RevertMigrations reverts the latest applied migration
func (m *Migrator) RevertMigration() (*MigrationResult, error) {
	return m.db.ApplyMigrations(MigrationDirectionDown, m.options)
}

// Status returns the status of all migrations
func (m *Migrator) Status() ([]*migrate.MigrationRecord, error) {
	return m.db.GetMigrationStatus(m.options)
}

// CreateMigration creates a new migration file
func (m *Migrator) CreateMigration(name string) (string, error) {
	return m.db.CreateMigration(name, m.options.Directory)
}

// PrintStatus prints the status of all migrations to the logger
func (m *Migrator) PrintStatus() error {
	migrations, err := m.db.FindMigrations(m.options.Directory)
	if err != nil {
		return err
	}

	records, err := m.db.GetMigrationStatus(m.options)
	if err != nil {
		return err
	}

	// Build a map of applied migrations
	applied := make(map[string]bool)
	for _, record := range records {
		applied[record.Id] = true
	}

	// Print status
	m.logger.Info("Migration status", zap.Int("total", len(migrations)))
	for _, migration := range migrations {
		status := "pending"
		if applied[migration.Id] {
			status = "applied"
		}
		m.logger.Info("Migration",
			zap.String("id", migration.Id),
			zap.String("status", status),
		)
	}

	return nil
}
